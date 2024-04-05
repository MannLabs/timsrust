use criterion::{black_box, criterion_group, criterion_main, Criterion};
use timsrust::FileReader;

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

fn criterion_benchmark(c: &mut Criterion) {
    // c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
    let mut group = c.benchmark_group("sample-size-example");
    group.significance_level(0.001).sample_size(10);
    let d_folder_name: &str = "/home/sander/data/20230505_TIMS05_PaSk_MA_HeLa_6min_ddaP_S1-C10_1_2323.d/";
    let file_reader: FileReader =
        FileReader::new(d_folder_name.to_string()).unwrap();
    group.bench_function("read_all_frames 6m dda", |b| {
        b.iter(|| read_all_frames(black_box(&file_reader)))
    });
    group.bench_function("read_all_ms1_frames 6m dda", |b| {
        b.iter(|| read_all_ms1_frames(black_box(&file_reader)))
    });
    group.bench_function("read_all_ms2_frames 6m dda", |b| {
        b.iter(|| read_all_ms2_frames(black_box(&file_reader)))
    });
    group.bench_function("read_all_spectra 6m dda", |b| {
        b.iter(|| read_all_spectra(black_box(&file_reader)))
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
