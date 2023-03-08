use anyhow::{anyhow, Error};
use derive_more::Error as ErrorFactory;
use std::fmt::{self, Debug};

pub trait Request: Debug + Sized + Send + 'static {
    type Response: Response<Request = Self>;

    fn function_code(&self) -> u8; // 1 - 127 (incl.)
    fn data(&self) -> Box<[u8]>; // of size 0 - 253 (incl.)
}

pub trait Response: Debug + Sized + Send + 'static {
    type Request: Request<Response = Self>;

    fn from_data(
        request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error>;
}

#[derive(ErrorFactory, Debug)]
pub struct Exception {
    code: u8,
}
impl Exception {
    pub fn new(code: u8) -> Self {
        Self { code }
    }
    pub fn from_data(data: &[u8]) -> Result<Option<Self>, Error> {
        match data {
            [] => Ok(None),
            [code] => Ok(Some(Self::new(*code))),
            _ => Err(anyhow!("frame size exceeded")),
        }
    }
}
impl fmt::Display for Exception {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "Exception (code = 2)")
    }
}
