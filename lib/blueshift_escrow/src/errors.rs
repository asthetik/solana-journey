use pinocchio::error::ProgramError;
use core::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowError {
    NotRentExempt = 0,

    NotSigner = 1,

    InvalidOwner = 2,

    InvalidAccountData = 3,

    InvalidAddress = 4,
}

impl From<EscrowError> for ProgramError {
    fn from(error: EscrowError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

impl fmt::Display for EscrowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EscrowError::NotRentExempt => write!(f, "Lamport balance below rent-exempt threshold"),
            EscrowError::NotSigner => write!(f, "没有签名"),
            EscrowError::InvalidOwner => write!(f, "非法的所有者"),
            EscrowError::InvalidAccountData => write!(f, "非法的账户数据"),
            EscrowError::InvalidAddress => write!(f, "非法的地址"),
        }
    }
}
