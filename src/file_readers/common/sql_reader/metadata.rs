use rusqlite::{Connection, Statement};

use crate::domain_converters::{
    Frame2RtConverter, Scan2ImConverter, Tof2MzConverter,
};

use super::{get_sql_connection, ReadableFromSql, SqlReader};

const OTOF_CONTROL: &str = "Bruker otofControl";

fn read_software(connection: &Connection) -> String {
    let software: String = connection
        .query_row(
            "SELECT Value FROM GlobalMetadata WHERE Key = 'AcquisitionSoftware'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    software
}

fn read_tof_max_index(connection: &Connection) -> u32 {
    let tof_max_index_string: String = connection
        .query_row(
            "SELECT Value FROM GlobalMetadata WHERE Key = 'DigitizerNumSamples'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut tof_max_index: u32 = tof_max_index_string.parse().unwrap();
    tof_max_index
}

fn read_mz_max_value(connection: &Connection) -> f64 {
    let mz_max_value_string: String = connection
        .query_row(
            "SELECT Value FROM GlobalMetadata WHERE Key = 'MzAcqRangeUpper'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut mz_max_value: f64 = mz_max_value_string.parse().unwrap();
    if read_software(connection) == OTOF_CONTROL {
        mz_max_value += 5.0;
    }
    mz_max_value
}

fn read_mz_min_value(connection: &Connection) -> f64 {
    let mz_min_value_string: String = connection
        .query_row(
            "SELECT Value FROM GlobalMetadata WHERE Key = 'MzAcqRangeLower'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut mz_min_value: f64 = mz_min_value_string.parse().unwrap();
    if read_software(connection) == OTOF_CONTROL {
        mz_min_value -= 5.0;
    }
    mz_min_value
}

impl SqlReader {
    fn read_metadata(&self, value_name: &str) -> String {
        let connection: Connection = get_sql_connection(&self.path);
        let query: String = format!(
            "SELECT Value FROM GlobalMetadata WHERE Key = '{}'",
            value_name
        );
        let mut stmt: Statement = connection.prepare(&query).unwrap();
        let value_str: String = stmt.query_row([], |row| row.get(0)).unwrap();
        value_str
    }

    pub fn read_im_information(&self) -> (u32, f64, f64) {
        let lower_im_value: f64 = self
            .read_metadata("OneOverK0AcqRangeLower")
            .parse()
            .unwrap();
        let upper_im_value: f64 = self
            .read_metadata("OneOverK0AcqRangeUpper")
            .parse()
            .unwrap();
        let scan_max_index: u32 = 927;
        (scan_max_index, lower_im_value, upper_im_value)
    }

    pub fn read_mz_information(&self) -> (u32, f64, f64) {
        let connection: Connection = get_sql_connection(&self.path);
        let tof_max_index: u32 = read_tof_max_index(&connection);
        let lower_mz_value: f64 = read_mz_min_value(&connection);
        let upper_mz_value: f64 = read_mz_max_value(&connection);
        (tof_max_index, lower_mz_value, upper_mz_value)
    }
}

impl ReadableFromSql for Tof2MzConverter {
    fn from_sql(sql_reader: &SqlReader) -> Self {
        let (tof_max_index, mz_min, mz_max) = sql_reader.read_mz_information();
        Self::from_boundaries(mz_min, mz_max, tof_max_index)
    }
}

impl ReadableFromSql for Scan2ImConverter {
    fn from_sql(sql_reader: &SqlReader) -> Self {
        let (scan_max_index, im_min, im_max) = sql_reader.read_im_information();
        Self::from_boundaries(im_min, im_max, scan_max_index)
    }
}

impl ReadableFromSql for Frame2RtConverter {
    fn from_sql(sql_reader: &SqlReader) -> Self {
        Self::from_values(sql_reader.read_column_from_table("Time", "Frames"))
    }
}
