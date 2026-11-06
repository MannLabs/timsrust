use timsrust::{
    NativeNeighborhoodReader,
    centroid::neighborhood::{
        AcquisitionPolicy, Anchor, Config, RtConsistencyConfig,
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(5..=6).contains(&args.len()) || (args.len() == 6 && args[5] != "--rt")
    {
        return Err("usage: native_observations RUN.d FRAME INTERVAL SCAN PRECURSOR_MZ [--rt]".into());
    }
    let reader = NativeNeighborhoodReader::new(
        &args[0],
        Config {
            acquisition: AcquisitionPolicy::ExperimentalTrajectory,
            rt_consistency: (args.len() == 6)
                .then(RtConsistencyConfig::default),
            ..Config::default()
        },
    )?;
    let bundle = reader.neighborhood_for_precursor(
        Anchor {
            frame: args[1].parse()?,
            interval: args[2].parse()?,
            scan: args[3].parse()?,
        },
        args[4].parse()?,
    )?;
    println!("{:#?}", reader.resolved_config());
    println!("{:#?}", reader.observation_domain(&bundle.focal)?);
    println!("{bundle:#?}");
    Ok(())
}
