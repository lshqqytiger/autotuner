use autotuner::parameter::{Instance, Profile, Value};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) fn random(profile: &Profile) -> Instance {
    Instance::new(
        profile
            .0
            .iter()
            .map(|(name, parameter)| (name.clone(), parameter.random()))
            .collect::<BTreeMap<Arc<str>, Value>>(),
    )
}

pub(crate) fn crossover(profile: &Profile, a: &Instance, b: &Instance) -> Instance {
    let mut parameters = BTreeMap::new();
    for parameter in &a.parameters {
        parameters.insert(
            parameter.0.clone(),
            profile
                .0
                .get(parameter.0)
                .unwrap()
                .crossover(&a.parameters[parameter.0], &b.parameters[parameter.0]),
        );
    }
    Instance::new(parameters)
}

pub(crate) fn mutate(profile: &Profile, instance: &mut Instance) {
    for (name, parameter) in &mut instance.parameters {
        profile.0.get(name).unwrap().mutate(parameter);
    }
}

pub(crate) fn stochastic_universal_sampling(roulette: &[(f64, usize)], n: usize) -> Vec<usize> {
    assert!(!roulette.is_empty());
    assert_ne!(n, 0);

    assert!(roulette.iter().all(|(f, _)| *f >= 0.0));

    let total_fitness: f64 = roulette.iter().map(|(fitness, _)| fitness).sum();
    assert!(total_fitness > 0.0);

    let distance = total_fitness / n as f64;
    let start = rand::random::<f64>() * distance;

    let mut selected = Vec::with_capacity(n);

    let mut current_sum = 0.0;
    let mut current_index = 0usize;

    for i in 0..n {
        let pointer = start + i as f64 * distance;

        while current_index < roulette.len() && current_sum < pointer {
            current_sum += roulette[current_index].0;
            current_index += 1;
        }

        if current_index == 0 {
            selected.push(roulette[0].1);
        } else if current_index <= roulette.len() {
            selected.push(roulette[current_index - 1].1);
        } else {
            selected.push(roulette[roulette.len() - 1].1);
        }
    }

    selected
}
