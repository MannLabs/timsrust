use criterion::{black_box, criterion_group, criterion_main, Criterion};
use timsrust::FileReader;

const DDA_TEST: &str =
    "/mnt/c/Users/Sander.Willems/Documents/data/tims05_300SPD/20230505_TIMS05_PaSk_MA_HeLa_6min_ddaP_S1-C10_1_2323.d/";
const DIA_TEST: &str =
    "/mnt/c/Users/Sander.Willems/Documents/data/20230505_TIMS05_PaSk_SA_HeLa_6min_diaP_8scans_S1-D3_1_2329.d/";
const SYP_TEST: &str =
    "/mnt/c/Users/Sander.Willems/Documents/data/20230505_TIMS05_PaSk_SA_HeLa_6min_syP_5scans_30Da_S1-D4_1_2330.d/";

fn read_all_frames(file_reader: &FileReader) {
    file_reader.read_all_frames();
}

fn read_all_ms1_frames(file_reader: &FileReader) {
    file_reader.read_all_ms1_frames();
}

fn read_all_ms2_frames(file_reader: &FileReader) {
    file_reader.read_all_ms2_frames();
}

fn read_all_spectra(file_reader: &FileReader) {
    file_reader.read_all_spectra();
}

fn criterion_benchmark_dda(c: &mut Criterion) {
    // c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
    let mut group = c.benchmark_group("sample-size-example");
    group.significance_level(0.001).sample_size(10);
    let d_folder_name: &str = DDA_TEST;
    let file_reader: FileReader =
        FileReader::new(d_folder_name.to_string()).unwrap();
    group.bench_function("DDA read_all_frames 6m", |b| {
        b.iter(|| read_all_frames(black_box(&file_reader)))
    });
    group.bench_function("DDA read_all_ms1_frames 6m", |b| {
        b.iter(|| read_all_ms1_frames(black_box(&file_reader)))
    });
    group.bench_function("DDA read_all_ms2_frames 6m", |b| {
        b.iter(|| read_all_ms2_frames(black_box(&file_reader)))
    });
    group.bench_function("DDA read_all_spectra 6m", |b| {
        b.iter(|| read_all_spectra(black_box(&file_reader)))
    });
    group.finish();
}

fn criterion_benchmark_dia(c: &mut Criterion) {
    // c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
    let mut group = c.benchmark_group("sample-size-example");
    group.significance_level(0.001).sample_size(10);
    let d_folder_name: &str = DIA_TEST;
    let file_reader: FileReader =
        FileReader::new(d_folder_name.to_string()).unwrap();
    group.bench_function("DIA read_all_frames 6m", |b| {
        b.iter(|| read_all_frames(black_box(&file_reader)))
    });
    group.bench_function("DIA read_all_ms1_frames 6m", |b| {
        b.iter(|| read_all_ms1_frames(black_box(&file_reader)))
    });
    group.bench_function("DIA read_all_ms2_frames 6m", |b| {
        b.iter(|| read_all_ms2_frames(black_box(&file_reader)))
    });
    group.finish();
}

fn criterion_benchmark_syp(c: &mut Criterion) {
    // c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
    let mut group = c.benchmark_group("sample-size-example");
    group.significance_level(0.001).sample_size(10);
    let d_folder_name: &str = SYP_TEST;
    let file_reader: FileReader =
        FileReader::new(d_folder_name.to_string()).unwrap();
    group.bench_function("SYP read_all_frames 6m", |b| {
        b.iter(|| read_all_frames(black_box(&file_reader)))
    });
    group.bench_function("SYP read_all_ms1_frames 6m", |b| {
        b.iter(|| read_all_ms1_frames(black_box(&file_reader)))
    });
    group.bench_function("SYP read_all_ms2_frames 6m", |b| {
        b.iter(|| read_all_ms2_frames(black_box(&file_reader)))
    });
    group.finish();
}

criterion_group!(
    benches,
    criterion_benchmark_dda,
    // criterion_benchmark_dia,
    // criterion_benchmark_syp
);
criterion_main!(benches);
