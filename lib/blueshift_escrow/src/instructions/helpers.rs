use pinocchio::{AccountView, Address, ProgramResult};
use pinocchio::cpi::{Seed, Signer};
use pinocchio::error::ProgramError;
use pinocchio::sysvars::rent::Rent;
use pinocchio::sysvars::Sysvar;
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_system::instructions::CreateAccount;
use crate::errors::EscrowError;


pub trait AccountCheck {
    fn check(account: &AccountView) -> Result<(), ProgramError>;
}

pub trait AssociatedTokenAccountCheck{
    fn check(
        account: &AccountView,
        authority: &AccountView,
        mint: &AccountView,
        token_program: &AccountView,
    ) -> Result<(), ProgramError>;
}

pub trait AssociatedTokenAccountInit{
    fn init(
        account: &AccountView,
        mint: &AccountView,
        payer: &AccountView,
        owner: &AccountView,
        system_program: &AccountView,
        token_program: &AccountView,
    ) -> ProgramResult;

    fn init_if_needed(
        account: &AccountView,
        mint: &AccountView,
        payer: &AccountView,
        owner: &AccountView,
        system_program: &AccountView,
        token_program: &AccountView,
    ) -> ProgramResult;
}

pub struct SignerAccount;

impl AccountCheck for SignerAccount {
    fn check(account: &AccountView) -> Result<(), ProgramError> {
        if !account.is_signer() {
            return Err(EscrowError::NotSigner.into());
        }
        Ok(())
    }
}

pub struct SystemAccount;

impl AccountCheck for SystemAccount {
    fn check(account: &AccountView) -> Result<(), ProgramError> {
        if !account.owned_by(&pinocchio_system::ID) {
            return Err(EscrowError::InvalidOwner.into());
        }

        Ok(())
    }
}

// Token-2022 Program ID
// TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
pub const TOKEN_2022_PROGRAM_ID: Address = Address::new_from_array(
    [
    0x06, 0xdd, 0xf6, 0xe1, 0xee, 0x75, 0x8f, 0xde, 0x18, 0x42, 0x5d, 0xbc, 0xe4, 0x6c, 0xcd, 0xda,
    0xb6, 0x1a, 0xfc, 0x4d, 0x83, 0xb9, 0x0d, 0x27, 0xfe, 0xbd, 0xf9, 0x28, 0xd8, 0xa1, 0x8b, 0xfc,
]);

const TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET: usize = 165;

pub const TOKEN_2022_MINT_DISCRIMINATOR: u8 = 0x01;

pub const TOKEN_2022_TOKEN_ACCOUNT_DISCRIMINATOR: u8 = 0x02;

pub struct MintInterface;

impl AccountCheck for MintInterface {
    fn check(account: &AccountView) -> Result<(), ProgramError> {
        if !account.owned_by(&TOKEN_2022_PROGRAM_ID) {
            if !account.owned_by(&pinocchio_token::ID) {
                return Err(EscrowError::InvalidOwner.into());
            } else {
                if account.data_len().ne(&pinocchio_token::state::Mint::LEN) {
                    return Err(EscrowError::InvalidAccountData.into());
                }
            }
        } else {
            let data = account.try_borrow()?;

            if data.len().ne(&pinocchio_token::state::Mint::LEN) {
                if data.len().le(&TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET) {
                    return Err(EscrowError::InvalidAccountData.into());
                }
                if data[TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET].ne(&TOKEN_2022_MINT_DISCRIMINATOR) {
                    return Err(EscrowError::InvalidAccountData.into());
                }
            }
        }

        Ok(())
    }
}

pub struct TokenAccountInterface;

impl AccountCheck for TokenAccountInterface {
    fn check(account: &AccountView) -> Result<(), ProgramError> {
        if !account.owned_by(&TOKEN_2022_PROGRAM_ID) {
            if !account.owned_by(&pinocchio_token::ID) {
                return Err(EscrowError::InvalidOwner.into());
            } else {
                if account.data_len().ne(&pinocchio_token::state::TokenAccount::LEN) {
                    return Err(EscrowError::InvalidAccountData.into());
                }
            }
        } else {
            let data = account.try_borrow()?;

            if data.len().ne(&pinocchio_token::state::TokenAccount::LEN) {
                if data.len().le(&TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET) {
                    return Err(EscrowError::InvalidAccountData.into());
                }
                if data[TOKEN_2022_ACCOUNT_DISCRIMINATOR_OFFSET]
                    .ne(&TOKEN_2022_TOKEN_ACCOUNT_DISCRIMINATOR)
                {
                    return Err(EscrowError::InvalidAccountData.into());
                }
            }
        }

        Ok(())
    }
}


pub struct AssociatedTokenAccount;

impl AssociatedTokenAccountCheck for AssociatedTokenAccount {
    fn check(
        account: &AccountView,
        authority: &AccountView,
        mint: &AccountView,
        token_program: &AccountView,
    ) -> Result<(), ProgramError> {
        TokenAccountInterface::check(account)?;

        let (pda, _bump) = Address::find_program_address(
            &[
                authority.address().as_ref(),
                token_program.address().as_ref(),
                mint.address().as_ref(),
            ],
            &pinocchio_associated_token_account::ID,
        );

        let pda_address = Address::new_from_array(pda.to_bytes());

        if pda_address.ne(account.address()) {
            return Err(EscrowError::InvalidAddress.into());
        }

        Ok(())
    }
}

impl AssociatedTokenAccountInit for AssociatedTokenAccount {
    fn init(
        account: &AccountView,
        mint: &AccountView,
        payer: &AccountView,
        owner: &AccountView,
        system_program: &AccountView,
        token_program: &AccountView,
    ) -> ProgramResult {
        Create {
            funding_account: payer,
            account,
            wallet: owner,
            mint,
            system_program,
            token_program,
        }.invoke()
    }

    fn init_if_needed(
        account: &AccountView,
        mint: &AccountView,
        payer: &AccountView,
        owner: &AccountView,
        system_program: &AccountView,
        token_program: &AccountView,
    ) -> ProgramResult {
        match Self::check(account, owner, mint, token_program) {
            Ok(_) => Ok(()),  // 账户已存在且正确，跳过创建
            Err(_) => Self::init(account, mint, payer, owner, system_program, token_program),  // 创建账户
        }
    }
}

pub struct ProgramAccount;

impl AccountCheck for ProgramAccount {
    fn check(account: &AccountView) -> Result<(), ProgramError> {
        if !account.owned_by(&crate::ID) {
            return Err(EscrowError::InvalidOwner.into());
        }
        if account.data_len().ne(&crate::state::Escrow::LEN) {
            return Err(EscrowError::InvalidAccountData.into());
        }

        Ok(())
    }
}

pub trait ProgramAccountInit {
   
    fn init<'a, T: Sized>(
        payer: &AccountView,
        account: &AccountView,
        seeds: &[Seed<'a>],
        space: usize,
    ) -> ProgramResult;
}

impl ProgramAccountInit for ProgramAccount {
    fn init<'a, T: Sized>(
        payer: &AccountView,
        account: &AccountView,
        seeds: &[Seed<'a>],
        space: usize,
    ) -> ProgramResult {
       
        let lamports = Rent::get()?.try_minimum_balance(space)?;

        let signer = [Signer::from(seeds)];
        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: space as u64,
            owner: &crate::ID,
        }
            .invoke_signed(&signer)?;

        Ok(())
    }
}


pub trait AccountClose {
    fn close(account: &AccountView, destination: &AccountView) -> ProgramResult;
}

impl AccountClose for ProgramAccount {
    fn close(account: &AccountView, destination: &AccountView) -> ProgramResult {
        {
            let mut data = account.try_borrow_mut()?;
            data[0] = 0xff;
        }

        destination.set_lamports(destination.lamports()+account.lamports());

        account.resize(1)?;

        account.close()
    }
}