use criterion::{criterion_group, criterion_main, Criterion};
use lib_game_detector::{data::SupportedLaunchers, get_detector};

fn main_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("main");

    group.bench_function("get_detector", |b| b.iter(get_detector));

    group.bench_function("get_detected_launchers", |b| {
        b.iter(|| get_detector().get_detected_launchers())
    });

    group.bench_function("get_all_detected_games", |b| {
        b.iter(|| get_detector().get_all_detected_games())
    });

    group.bench_function("get_all_detected_games_with_box_art", |b| {
        b.iter(|| get_detector().get_all_detected_games_with_box_art())
    });

    group.finish();
}

fn per_launcher_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("per_launcher");
    let detector = get_detector();

    group.bench_function("steam", |b| {
        b.iter(|| detector.get_all_detected_games_from_specific_launcher(SupportedLaunchers::Steam))
    });

    group.bench_function("heroic - epic", |b| {
        b.iter(|| {
            detector.get_all_detected_games_from_specific_launcher(
                SupportedLaunchers::HeroicGamesEpicGames,
            )
        })
    });

    group.bench_function("heroic - gog", |b| {
        b.iter(|| {
            detector.get_all_detected_games_from_specific_launcher(SupportedLaunchers::HeroicGOG)
        })
    });

    group.bench_function("heroic - amazon", |b| {
        b.iter(|| {
            detector.get_all_detected_games_from_specific_launcher(
                SupportedLaunchers::HeroicGamesAmazon,
            )
        })
    });

    group.bench_function("lutris", |b| {
        b.iter(|| {
            detector.get_all_detected_games_from_specific_launcher(SupportedLaunchers::Lutris)
        })
    });

    group.bench_function("bottles", |b| {
        b.iter(|| {
            detector.get_all_detected_games_from_specific_launcher(SupportedLaunchers::Bottles)
        })
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = main_benchmarks, per_launcher_benchmark
}
criterion_main!(benches);
