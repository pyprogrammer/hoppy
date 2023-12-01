use dam::{
    context_tools::{Receiver, Sender},
    types::DAMType,
};
use pyo3::prelude::*;

pub enum Stream<T: DAMType> {
    Sender(Sender<T>),
    Receiver(Receiver<T>),
}

macro_rules! StreamEnum {
    ($($t:tt),*) => {
        #[allow(non_camel_case_types)]
        pub enum StreamEnum {
            $(
                $t(Stream<$t>),
            )*
        }
    };
}

StreamEnum!(u16, u32);

#[pyclass]
struct HopStream {
    stream: StreamEnum,
}
