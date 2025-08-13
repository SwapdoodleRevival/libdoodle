use crate::{bpk1::BPK1File, error::GenericError, files::stationery::Stationery};

impl TryFrom<&[u8]> for Stationery {
    type Error = GenericError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Stationery::new_from_bpk1_bytes(value)
    }
}
