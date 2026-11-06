use std::{
    collections::HashMap,
    process::Command,
    time::{Duration, Instant, SystemTime},
};

use rand::{SeedableRng, rngs::StdRng};

const SEED_VAR: &str = "MAD_TURMOIL_TEST_SEED";

fn main() {
    assert_real_clocks();
    match std::env::var(SEED_VAR) {
        Ok(seed) => simulate(seed.parse().expect("seed")),
        Err(_) => {
            let first = observe(1);
            assert!(!first.is_empty());
            assert_eq!(first, observe(1), "same seed must reproduce");
            let second = observe(2);
            assert_eq!(first.len(), second.len());
            for (a, b) in first.iter().zip(&second) {
                assert_ne!(a, b, "different seeds must diverge");
            }
            println!("determinism: ok");
        }
    }
}

fn observe(seed: u64) -> Vec<String> {
    let out = Command::new(std::env::current_exe().expect("current exe"))
        .env(SEED_VAR, seed.to_string())
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "seed {seed} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf8")
        .lines()
        .map(String::from)
        .collect()
}

fn assert_real_clocks() {
    let since_epoch = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("after epoch");
    assert!(since_epoch > Duration::from_secs(1_700_000_000));
    let start = Instant::now();
    std::thread::sleep(Duration::from_millis(5));
    assert!(start.elapsed() >= Duration::from_millis(5));
}

fn simulate(seed: u64) {
    mad_turmoil::rand::set_rng(StdRng::seed_from_u64(seed));
    let guard = mad_turmoil::time::SimClocksGuard::init();

    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("getrandom");
    println!("getrandom: {bytes:?}");

    let map: HashMap<u32, ()> = (0..32).map(|i| (i, ())).collect();
    println!("hashmap: {:?}", map.keys().collect::<Vec<_>>());

    let epoch = SystemTime::UNIX_EPOCH + Duration::from_secs(1_750_000_000);
    let mut sim = turmoil::Builder::new().rng_seed(seed).epoch(epoch).build();
    sim.client("client", async move {
        let start = Instant::now();
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert_eq!(start.elapsed(), Duration::from_secs(3));
        assert_eq!(
            SystemTime::now().duration_since(epoch)?,
            Duration::from_secs(3)
        );
        Ok(())
    });
    sim.run().expect("sim");

    drop(guard);
    assert_real_clocks();
}
