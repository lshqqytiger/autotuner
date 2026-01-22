# Auto-tuner

TBD

## Notes

### [Example] Project Structure A

- Initializer, Evaluator, Finalizer, ...
- Kernel Source

Run with _Kernel Source_ and _Helper_.

### [Example] Project Structure B

- Initializer, Finalizer, ...
- Kernel Generator (hook)

Run with _Helper_.

1. Call `append_source()` at _Kernel Generator_.
2. Insert _Evaluator_ within the generated kernel.
