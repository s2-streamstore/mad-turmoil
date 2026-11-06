# mad-turmoil

[madsim](https://github.com/madsim-rs/madsim)-inspired determinism for [turmoil](https://github.com/tokio-rs/turmoil)-based simulation tests

See [blog post](https://s2.dev/blog/dst)

## Setup

Make sure you are only depending on this crate for simulation binaries!

```rust
fn main() -> eyre::Result<()> {
    let rng_seed = std::env::var("DST_SEED")?.parse()?;

    // Taming randomness...

    let mut rng = StdRng::seed_from_u64(rng_seed);
    mad_turmoil::rand::set_rng(rng.clone());
    assert_eq!(rng.next_u64(), mad_turmoil::rand::get_rng().next_u64());

    // Additionally, if you are using fastrand (possibly transitively!)
    fastrand::seed(rng_seed);

    // Taming time...

    let _tokio_time_guard = mad_turmoil::time::SimClocksGuard::init();

    // Go forth and create turmoil...
}
```

## Wall-clock time

Inside a simulation, `SystemTime::now()` returns turmoil's logical time since its configured epoch.
turmoil defaults the epoch to `SystemTime::now()` at builder creation, which mad-turmoil pins to the Unix epoch, so you may want to set a realistic one:

```rust
let sim = turmoil::Builder::new()
    .epoch(SystemTime::UNIX_EPOCH + Duration::from_secs(1_750_000_000))
    .build();
```

Clocks are only overridden while a `SimClocksGuard` is alive; outside of it, `clock_gettime` behaves normally.

## Tokio's `parking_lot` feature

Avoid enabling tokio's `parking_lot` feature (which `full` also enables) in simulation builds.
With it, tokio's clock sits behind a `parking_lot` mutex whose contended slow path reads the clock while holding internal locks, which can deadlock once anything touches the runtime from another thread, e.g. `spawn_blocking`.
Check with `cargo tree -e features -i tokio | grep parking_lot`.
