use pinocchio::{
    error::ProgramError,
    AccountView,
    ProgramResult,
};
use pinocchio_system::instructions::Transfer;
use core::mem::size_of;

pub fn deposit(data: &[u8], accounts: &[AccountView]) -> ProgramResult {
    if data.len() != size_of::<u64>() {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(data.try_into().unwrap());
    if amount == 0 {
        return Err(ProgramError::InsufficientFunds);
    }
    
    let [owner, vault, _system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    
    if !owner.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !vault.owned_by(&pinocchio_system::ID) {
        return Err(ProgramError::InvalidAccountOwner);
    }
    
    if vault.lamports() != 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    
    let (vault_key, _) = pinocchio::Address::find_program_address(&[b"vault", owner.address().as_ref()], &crate::ID);
    if vault.address() != &vault_key {
        return Err(ProgramError::InvalidSeeds);
    }

    Transfer {
        from: owner,
        to: vault,
        lamports: amount,
    }
    .invoke()?;
    
    Ok(())
}