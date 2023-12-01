use dam::context_tools::*;
use dam::simulation::ProgramBuilder;
use dam::utility_contexts::GeneratorContext;
use pyo3::prelude::*;

pub enum Stream<T: DAMType> {
    Receiver(Receiver<T>),
    None,
}

impl<T: DAMType> From<Receiver<T>> for Stream<T> {
    fn from(value: Receiver<T>) -> Self {
        Self::Receiver(value)
    }
}

macro_rules! StreamEnum {
    ($($t:tt),*) => {
        #[allow(non_camel_case_types)]
        pub enum StreamEnum {
            $(
                $t(Stream<$t>),
            )*
        }

        impl StreamEnum {
            pub fn from_constant(builder: &mut ProgramBuilder<'_>, values: &PyAny) -> Option<Self> {
                $(
                    if let Ok(v) = values.extract::<Vec<$t>>() {
                        let (snd, rcv) = builder.unbounded();
                        builder.add_child(GeneratorContext::new(|| v.into_iter(), snd));
                        return Some(Self::$t(rcv.into()));
                    }
                )*
                None
            }

            pub fn into_list(&mut self, builder: &mut ProgramBuilder<'_>) -> Box<dyn Fn(Python<'_>) -> Py<PyAny>> {
                match self {
                    $(
                        StreamEnum::$t(stream) => {
                            if let Stream::Receiver(recv) = std::mem::replace(stream, Stream::None) {
                                let (ctx, out) = CollectorContext::new(recv);
                                builder.add_child(ctx);

                                return Box::new(move |py| {
                                    out.lock().map(|lockguard| {
                                        lockguard.clone().into_py(py)
                                    }).unwrap_or_else(|_| py.None())
                                })
                            } else {
                                panic!("Attempted to call into_list on a stream multiple times.");
                            }
                        }
                    )*
                }
            }
        }
    };
}

StreamEnum!(i64, f64);

#[pyclass]
pub struct HopStream {
    pub stream: StreamEnum,
}

#[context_macro]
struct CollectorContext<T: DAMType> {
    rcv: Receiver<T>,
    output: std::sync::Arc<std::sync::Mutex<Vec<T>>>,
}

impl<T: DAMType> CollectorContext<T> {
    pub fn new(rcv: Receiver<T>) -> (Self, std::sync::Arc<std::sync::Mutex<Vec<T>>>) {
        let ctx = Self {
            rcv,
            output: Default::default(),
            context_info: Default::default(),
        };
        ctx.rcv.attach_receiver(&ctx);
        let out = ctx.output.clone();
        (ctx, out)
    }
}

impl<T: DAMType> Context for CollectorContext<T> {
    fn run(&mut self) {
        let mut out = self.output.lock().unwrap();
        loop {
            match self.rcv.dequeue(&self.time) {
                Ok(ChannelElement { time: _, data }) => {
                    out.push(data);
                }
                Err(_) => return,
            }
        }
    }
}
