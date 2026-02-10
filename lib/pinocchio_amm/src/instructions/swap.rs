use constant_product_curve::{ConstantProduct, LiquidityPair};
use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock},
};
use pinocchio_token::{instructions::Transfer, state::TokenAccount};

use crate::Config;

/*
    计算通过将一定数量的 mint_y 发送到 AMM（或反之）后，能够接收到的 mint_x 的数量，包括手续费。

    将 from 代币转移到金库，并将 to 代币转移到用户的代币账户。
*/
pub struct SwapAccounts<'a> {
    pub user: &'a AccountView,
    pub user_x_ata: &'a AccountView,
    pub user_y_ata: &'a AccountView,
    pub vault_x: &'a AccountView,
    pub vault_y: &'a AccountView,
    pub config: &'a AccountView,
    pub token_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for SwapAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let mut iter = accounts.iter();
        Ok(Self {
            user: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            user_x_ata: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            user_y_ata: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            vault_x: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            vault_y: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            config: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
            token_program: iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?,
        })
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SwapInstructionData {
    pub is_x: bool,
    pub amount: u64,
    pub min: u64,
    pub expiration: i64,
}

impl<'a> TryFrom<&'a [u8]> for SwapInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < size_of::<Self>() {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(unsafe { *(data.as_ptr() as *const Self) })
    }
}

pub struct Swap<'a> {
    pub accounts: SwapAccounts<'a>,
    pub instruction_data: SwapInstructionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Swap<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let accounts = SwapAccounts::try_from(accounts)?;
        let instruction_data = SwapInstructionData::try_from(data)?;

        // Return the initialized struct
        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}
impl<'a> Swap<'a> {
    pub const DISCRIMINATOR: &'a u8 = &3;

    pub fn process(&mut self) -> ProgramResult {
        let accounts = &self.accounts;
        let data = &self.instruction_data;

        // 1. 验证过期时间
        let clock = Clock::get()?;
        if clock.unix_timestamp > data.expiration {
            return Err(ProgramError::InvalidArgument);
        }

        // 2. 加载配置和状态
        let config = Config::load(accounts.config)?;
        if config.state() != 1 {
            // 必须是 Initialized
            return Err(ProgramError::InvalidAccountData);
        }

        // 3. 获取金库当前余额并计算交换
        let vault_x = unsafe { TokenAccount::from_account_view_unchecked(accounts.vault_x)? };
        let vault_y = unsafe { TokenAccount::from_account_view_unchecked(accounts.vault_y)? };

        let mut curve = ConstantProduct::init(
            vault_x.amount(),
            vault_y.amount(),
            vault_x.amount(), // 这里 supply 通常用于初始价格，交换中主要看储备
            config.fee(),
            None,
        )
        .map_err(|_| ProgramError::ArithmeticOverflow)?;

        let pair = if data.is_x {
            LiquidityPair::X
        } else {
            LiquidityPair::Y
        };
        let swap_result = curve
            .swap(pair, data.amount, data.min)
            .map_err(|_| ProgramError::InvalidArgument)?;

        // 4. 准备签名种子 (用于从金库转出)
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

        // 5. 执行原子转账
        if data.is_x {
            // X -> Y: 用户发送 X 到 vault_x，金库发送 Y 到 user_y_ata
            Transfer {
                from: accounts.user_x_ata,
                to: accounts.vault_x,
                authority: accounts.user,
                amount: swap_result.deposit,
            }
            .invoke()?;

            Transfer {
                from: accounts.vault_y,
                to: accounts.user_y_ata,
                authority: accounts.config,
                amount: swap_result.withdraw,
            }
            .invoke_signed(&[signer])?;
        } else {
            // Y -> X: 用户发送 Y 到 vault_y，金库发送 X 到 user_x_ata
            Transfer {
                from: accounts.user_y_ata,
                to: accounts.vault_y,
                authority: accounts.user,
                amount: swap_result.deposit,
            }
            .invoke()?;

            Transfer {
                from: accounts.vault_x,
                to: accounts.user_x_ata,
                authority: accounts.config,
                amount: swap_result.withdraw,
            }
            .invoke_signed(&[signer])?;
        }

        Ok(())
    }
}