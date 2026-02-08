use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView,
    ProgramResult,
};
use pinocchio_system::instructions::Transfer;

pub fn withdraw(accounts: &[AccountView]) -> ProgramResult {
    let [owner, vault, _system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    
    if !owner.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !vault.owned_by(&pinocchio_system::ID) {
        return Err(ProgramError::InvalidAccountOwner);
    }
    
    if vault.lamports() == 0 {
        return Err(ProgramError::InsufficientFunds);
    }
    
    let (vault_key, bump) = pinocchio::Address::find_program_address(&[b"vault", owner.address().as_ref()], &crate::ID);
    if vault.address() != &vault_key {
        return Err(ProgramError::InvalidSeeds);
    }
    
    let bump_binding = [bump];
    let seeds = [
        Seed::from(b"vault"),
        Seed::from(owner.address().as_ref()),
        Seed::from(&bump_binding),
    ];
    let signers = [Signer::from(&seeds)];
    
    Transfer {
        from: vault,
        to: owner,
        lamports: vault.lamports(),
    }
    .invoke_signed(&signers)?;
    
    Ok(())
}