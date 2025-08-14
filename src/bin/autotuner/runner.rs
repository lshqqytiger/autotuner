use anyhow::anyhow;
use autotuner::{metadata::Metadata, parameter::Instance};
use libc::SIGSEGV;
use libloading::{Library, Symbol};
use signal_hook_registry::{register_unchecked, unregister};
use std::{
    ffi::{self, OsStr},
    process::{self, Command},
    ptr, thread,
};
use tempdir::TempDir;

type Initializer = unsafe extern "C" fn(
    arg_in: *mut *mut ffi::c_void,
    arg_out: *mut *mut ffi::c_void,
    arg_val: *mut *mut ffi::c_void,
);
type Finalizer = unsafe extern "C" fn(
    arg_in: *mut ffi::c_void,
    arg_out: *mut ffi::c_void,
    arg_val: *mut ffi::c_void,
);
type Evaluator = unsafe extern "C" fn(arg_in: *mut ffi::c_void, arg_out: *mut ffi::c_void) -> f64;
type Validator =
    unsafe extern "C" fn(arg_val: *const ffi::c_void, arg_out: *const ffi::c_void) -> bool;

// TODO: input_ptr and validation_ptr can be shared between threads
struct Workspace {
    input_ptr: *mut ffi::c_void, // const after initialization
    output_ptr: *mut ffi::c_void,
    validation_ptr: Option<*mut ffi::c_void>, // const after initialization
}

impl Workspace {
    fn new(lib: &Library, metadata: &Metadata) -> Result<Self, libloading::Error> {
        let initializer: Symbol<Initializer> = unsafe { lib.get(metadata.initializer.as_bytes()) }?;
        let mut input_ptr = ptr::null_mut();
        let mut output_ptr = ptr::null_mut();
        let mut validation_ptr = if metadata.validator.is_some() {
            Some(ptr::null_mut())
        } else {
            None
        };

        unsafe {
            initializer(
                &mut input_ptr,
                &mut output_ptr,
                if let Some(ptr) = validation_ptr.as_mut() {
                    ptr
                } else {
                    ptr::null_mut()
                },
            );
        }

        Ok(Workspace {
            input_ptr,
            output_ptr,
            validation_ptr,
        })
    }
}

unsafe impl Sync for Workspace {}

fn compile<S: AsRef<OsStr>>(
    compiler: &String,
    temp_dir: &TempDir,
    arguments: impl Iterator<Item = S>,
) -> anyhow::Result<Library> {
    let path = temp_dir
        .path()
        .join(thread::current().name().unwrap_or("temp"));
    let mut compiler = Command::new(compiler);
    let compiler = compiler.arg("-shared").arg("-o").arg(&path).args(arguments);

    let mut compiler = compiler.spawn()?;
    compiler.wait()?;

    let lib = unsafe { Library::new(&path) }?;
    Ok(lib)
}

pub(crate) struct Runner {
    sources: Vec<String>,
    metadata: Metadata,
    temp_dir: TempDir,
    base: Library,
    workspaces: Vec<Workspace>,
}

impl Runner {
    pub(crate) fn new(
        sources: Vec<String>,
        metadata: Metadata,
        parallelism: usize,
    ) -> anyhow::Result<Self> {
        let temp_dir = TempDir::new("autotuner")?;
        let workspaces = Vec::with_capacity(parallelism);
        let base = compile(
            &metadata.compiler,
            &temp_dir,
            sources.iter().chain(metadata.compiler_arguments.iter()),
        )?;
        let mut runner = Runner {
            sources,
            metadata,
            temp_dir,
            base,
            workspaces,
        };
        for _ in 0..parallelism {
            let workspace = Workspace::new(&runner.base, &runner.metadata)?;
            runner.workspaces.push(workspace);
        }
        Ok(runner)
    }

    pub(crate) fn evaluate(&self, instance: &Instance, repetition: usize) -> anyhow::Result<f64> {
        let lib = compile(
            &self.metadata.compiler,
            &self.temp_dir,
            self.sources
                .iter()
                .chain(self.metadata.compiler_arguments.iter())
                .chain(instance.compiler_arguments().iter()),
        )?;
        let evaluator: Symbol<Evaluator> = unsafe { lib.get(self.metadata.evaluator.as_bytes()) }?;

        let tid = rayon::current_thread_index().unwrap_or(0);
        let workspace = &self.workspaces[tid];
        let mut fitnesses = Vec::with_capacity(repetition);
        for _ in 0..repetition {
            let fitness = unsafe {
                let result = register_unchecked(SIGSEGV, |_| {
                    // can we do better than this?
                    println!("Segmentation fault occurred during evaluation");
                    process::exit(1);
                });
                let fitness = evaluator(workspace.input_ptr, workspace.output_ptr);
                if let Ok(id) = result {
                    unregister(id);
                }
                fitness
            };
            if fitness.is_nan() {
                return Err(anyhow!("NaN value encountered"));
            }
            fitnesses.push(fitness);
        }
        fitnesses.sort_by(|a, b| a.total_cmp(b));

        if let Some(block) = workspace.validation_ptr {
            let validator: Symbol<Validator> =
                unsafe { lib.get(self.metadata.validator.as_ref().unwrap().as_bytes()) }?;
            if !unsafe { validator(block, workspace.output_ptr) } {
                return Ok(f64::INFINITY);
            }
        }

        Ok(fitnesses[fitnesses.len() / 2])
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        if let None = self.metadata.finalizer {
            return;
        }

        let finalizer: Symbol<Finalizer> = if let Ok(symbol) = unsafe {
            self.base
                .get(self.metadata.finalizer.as_ref().unwrap().as_bytes())
        } {
            symbol
        } else {
            return;
        };

        for workspace in &self.workspaces {
            unsafe {
                finalizer(
                    workspace.input_ptr,
                    workspace.output_ptr,
                    if let Some(ptr) = workspace.validation_ptr {
                        ptr
                    } else {
                        ptr::null_mut()
                    },
                );
            }
        }
    }
}
