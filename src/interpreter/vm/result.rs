use thiserror::Error;

pub type InterpreterResult<T> = std::result::Result<T, InterpreterError>;

#[derive(Error, Debug)]
pub enum InterpreterError {
    #[error("compile error")]
    Compile,
    #[error("runtime error")]
    Runtime,
}
