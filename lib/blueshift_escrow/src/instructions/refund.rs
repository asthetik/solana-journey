use pinocchio::{AccountView, ProgramResult};
use pinocchio::cpi::{Seed, Signer};
use pinocchio::error::ProgramError;
use pinocchio_token::instructions::{CloseAccount, Transfer};
use solana_address::Address;
use crate::{AccountCheck, AccountClose, AssociatedTokenAccount, AssociatedTokenAccountInit, Escrow, MintInterface, ProgramAccount, SignerAccount};

pub struct RefundAccount<'info> {
    pub maker: &'info AccountView,

    pub escrow: &'info AccountView,

    pub mint_a: &'info AccountView,

    pub vault: &'info AccountView,

    pub maker_ata_a: &'info AccountView,

    pub system_program: &'info AccountView,

    pub token_program: &'info AccountView,
}

impl<'info> TryFrom<&'info [AccountView]> for RefundAccount<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountView]) -> Result<Self, Self::Error> {
        let [maker, escrow, mint_a, vault, maker_ata_a, system_program, token_program, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        SignerAccount::check(maker)?;

        ProgramAccount::check(escrow)?;

        MintInterface::check(mint_a)?;

        Ok(Self {
            maker,
            escrow,
            mint_a,
            vault,
            maker_ata_a,
            system_program,
            token_program,
        })
    }
}

pub struct Refund<'info> {
    pub accounts: RefundAccount<'info>,
}

impl<'info> TryFrom<&'info [AccountView]> for crate::Refund<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountView]) -> Result<Self, Self::Error> {
        let accounts = RefundAccount::try_from(accounts)?;

        AssociatedTokenAccount::init_if_needed(
            accounts.maker_ata_a,
            accounts.mint_a,
            accounts.maker,
            accounts.maker,
            accounts.system_program,
            accounts.token_program,
        )?;

        Ok(Self {
            accounts,
        })
    }
}

impl<'info> Refund<'info> {
    pub const DISCRIMINATOR: &'info u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        let (seed, bump) = {
            let data = self.accounts.escrow.try_borrow()?;

            let escrow = Escrow::load(&data)?;

            let escrow_key = Address::create_program_address(
                &[
                    b"escrow",
                    self.accounts.maker.address().as_ref(),
                    &escrow.seed.to_le_bytes(),
                    &escrow.bump,
                ],
                &crate::ID
            )?;

            if &escrow_key != self.accounts.escrow.address() {
                return Err(ProgramError::InvalidAccountOwner);
            }

            (escrow.seed, escrow.bump)
        };

        let seed_binding = seed.to_le_bytes();
        let bump_binding = bump;
        let escrow_seeds = [
            Seed::from(b"escrow"),
            Seed::from(self.accounts.maker.address().as_ref()),
            Seed::from(&seed_binding),
            Seed::from(&bump_binding),
        ];

        let signer = Signer::from(&escrow_seeds);

        let amount = {
            let vault_data = self.accounts.vault.try_borrow()?;

            u64::from_le_bytes(vault_data[64..72].try_into().unwrap())
        };

        Transfer {
            from: self.accounts.vault,
            to: self.accounts.maker_ata_a,
            authority: self.accounts.escrow,
            amount,
        }.invoke_signed(&[signer.clone()])?;

        CloseAccount {
            account: self.accounts.vault,
            destination: self.accounts.maker,
            authority: self.accounts.escrow,
        }.invoke_signed(&[signer.clone()])?;

        ProgramAccount::close(
            self.accounts.escrow,
            self.accounts.maker
        )?;

        Ok(())
    }
}
