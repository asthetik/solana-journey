use constant_product_curve::ConstantProduct;
use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock},
};
use pinocchio_token::{
    instructions::{Burn, Transfer},
    state::{Mint, TokenAccount},
};

use crate::Config;

/*
    根据用户希望 burn 的 LP 数量，提取 mint_x 和 mint_y 代币。

    计算提取金额，并检查金额是否不低于用户指定的 mint_x 和 mint_y。

    从用户的 ata 中销毁相应数量的 mint_lp。
*/

pub struct WithdrawAccounts<'a> {
    pub user: &'a AccountView,
    pub mint_lp: &'a AccountView,
    pub vault_x: &'a AccountView,
    pub vault_y: &'a AccountView,
    pub user_x_ata: &'a AccountView,
    pub user_y_ata: &'a AccountView,
    pub user_lp_ata: &'a AccountView,
    pub config: &'a AccountView,
    pub token_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for WithdrawAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let mut iter = accounts.iter();
        Ok(Self {
            user: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            mint_lp: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            vault_x: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            vault_y: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            user_x_ata: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            user_y_ata: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            user_lp_ata: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            config: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            token_program: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
        })
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct WithdrawInstructionData {
    pub amount: u64,
    pub min_x: u64,
    pub min_y: u64,
    pub expiration: i64,
}

impl<'a> TryFrom<&'a [u8]> for WithdrawInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < size_of::<Self>() {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(unsafe { *(data.as_ptr() as *const Self) })
    }
}

pub struct Withdraw<'a> {
    pub accounts: WithdrawAccounts<'a>,
    pub instruction_data: WithdrawInstructionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Withdraw<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let accounts = WithdrawAccounts::try_from(accounts)?;
        let instruction_data = WithdrawInstructionData::try_from(data)?;

        // Return the initialized struct
        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> Withdraw<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        let accounts = &self.accounts;
        let data = &self.instruction_data;

        // 1. 过期检查
        let clock = Clock::get()?;
        if clock.unix_timestamp > data.expiration {
            return Err(ProgramError::InvalidArgument);
        }

        // 2. 加载状态并检查 (Withdraw 要求非 Disabled)
        let config = Config::load(accounts.config)?;
        // 假设 0: Uninitialized, 1: Initialized, 2: Disabled
        if config.state() == 2 {
            return Err(ProgramError::InvalidAccountData);
        }

        // 3. 反序列化代币信息
        let mint_lp = unsafe { Mint::from_account_view_unchecked(accounts.mint_lp)? };
        let vault_x = unsafe { TokenAccount::from_account_view_unchecked(accounts.vault_x)? };
        let vault_y = unsafe { TokenAccount::from_account_view_unchecked(accounts.vault_y)? };

        // 4. 计算应退还的 X, Y 数量
        let (x, y) = if mint_lp.supply() == data.amount {
            // 全额提取：直接取走所有余额，防止舍入误差留下“尘埃”
            (vault_x.amount(), vault_y.amount())
        } else {
            let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
                vault_x.amount(),
                vault_y.amount(),
                mint_lp.supply(),
                data.amount,
                6, // LP decimals
            )
            .map_err(|_| ProgramError::ArithmeticOverflow)?;
            (amounts.x, amounts.y)
        };

        // 5. 滑点检查
        if x < data.min_x || y < data.min_y {
            return Err(ProgramError::InvalidArgument);
        }

        // 6. 销毁用户的 LP 代币 (用户签名)
        Burn {
            mint: accounts.mint_lp,
            account: accounts.user_lp_ata,
            authority: accounts.user,
            amount: data.amount,
        }
        .invoke()?;

        // 7. 构造 Config PDA 签名以从金库转账
        let seed_binding = config.seed().to_le_bytes();
        let mint_x_key = config.mint_x();
        let mint_y_key = config.mint_y();
        let bump = config.config_bump();

        let config_seeds = [
            Seed::from(b"config"),
            Seed::from(&seed_binding),
            Seed::from(mint_x_key.as_ref()),
            Seed::from(mint_y_key.as_ref()),
            Seed::from(&bump),
        ];
        let signer = Signer::from(&config_seeds);

        // 8. 转移 Token X 和 Y (Config PDA 签名)
        Transfer {
            from: accounts.vault_x,
            to: accounts.user_x_ata,
            authority: accounts.config,
            amount: x,
        }
        // .invoke_signed(&[signer.clone()])?;
        .invoke_signed(std::slice::from_ref(&signer))?;

        Transfer {
            from: accounts.vault_y,
            to: accounts.user_y_ata,
            authority: accounts.config,
            amount: y,
        }
        .invoke_signed(&[signer])?;

        Ok(())
    }
}