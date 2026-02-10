use constant_product_curve::ConstantProduct;
use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock},
};
use pinocchio_token::{
    instructions::{MintTo, Transfer},
    state::{Mint, TokenAccount},
};

use crate::Config;

pub struct DepositAccounts<'a> {
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

impl<'a> TryFrom<&'a [AccountView]> for DepositAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let mut account_iter = accounts.iter();
        Ok(Self {
            user: account_iter
                .next()
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
            mint_lp: account_iter
                .next()
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
            vault_x: account_iter
                .next()
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
            vault_y: account_iter
                .next()
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
            user_x_ata: account_iter
                .next()
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
            user_y_ata: account_iter
                .next()
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
            user_lp_ata: account_iter
                .next()
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
            config: account_iter
                .next()
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
            token_program: account_iter
                .next()
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
        })
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DepositInstructionData {
    pub amount: u64,
    pub max_x: u64,
    pub max_y: u64,
    pub expiration: i64,
}

impl<'a> TryFrom<&'a [u8]> for DepositInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < size_of::<Self>() {
            // 32 = 8 + 8 + 8 + 8
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(unsafe { *(data.as_ptr() as *const Self) })
    }
}

pub struct Deposit<'a> {
    pub accounts: DepositAccounts<'a>,
    pub instruction_data: DepositInstructionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Deposit<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let accounts = DepositAccounts::try_from(accounts)?;
        let instruction_data = DepositInstructionData::try_from(data)?;

        // Return the initialized struct
        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> Deposit<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1;

    pub fn process(&mut self) -> ProgramResult {
        let accounts = &self.accounts;
        let data = &self.instruction_data;

        // 1. 过期检查
        let clock = Clock::get()?;
        if clock.unix_timestamp > data.expiration {
            return Err(ProgramError::InvalidArgument); // 订单已过期
        }

        // 2. 加载 Config 并验证状态
        let config = Config::load(accounts.config)?;
        if config.state() != 1 {
            // AmmState::Initialized
            return Err(ProgramError::InvalidAccountData);
        }

        // 3. 反序列化代币账户信息 (使用 Pinocchio-token 提供的 unchecked 方法提升性能)
        let mint_lp = unsafe { Mint::from_account_view_unchecked(accounts.mint_lp)? };
        let vault_x = unsafe { TokenAccount::from_account_view_unchecked(accounts.vault_x)? };
        let vault_y = unsafe { TokenAccount::from_account_view_unchecked(accounts.vault_y)? };

        // 4. 计算存款金额 (x, y)
        let (x, y) = if mint_lp.supply() == 0 {
            // 初始流动性：使用用户指定的 max 值
            (data.max_x, data.max_y)
        } else {
            // 后续流动性：基于比例计算
            let amounts = ConstantProduct::xy_deposit_amounts_from_l(
                vault_x.amount(),
                vault_y.amount(),
                mint_lp.supply(),
                data.amount,
                6, // 假设 LP 小数位为 6
            )
            .map_err(|_| ProgramError::ArithmeticOverflow)?;
            (amounts.x, amounts.y)
        };

        // 5. 滑点保护检查
        if x > data.max_x || y > data.max_y {
            return Err(ProgramError::InvalidArgument);
        }

        // 6. 执行代币转移 (用户 -> 金库)
        Transfer {
            from: accounts.user_x_ata,
            to: accounts.vault_x,
            authority: accounts.user,
            amount: x,
        }
        .invoke()?;

        Transfer {
            from: accounts.user_y_ata,
            to: accounts.vault_y,
            authority: accounts.user,
            amount: y,
        }
        .invoke()?;

        // 7. 签署并执行 MintTo (Config PDA -> 用户)
        let seed_binding = config.seed().to_le_bytes();
        let mint_x = config.mint_x(); // Returns &Pubkey
        let mint_y = config.mint_y(); // Returns &Pubkey
        let bump = config.config_bump(); // Returns [u8; 1]

        // 2. Now create the seeds using references to those stable variables
        let config_seeds = [
            Seed::from(b"config"),
            Seed::from(&seed_binding),
            Seed::from(mint_x.as_ref()),
            Seed::from(mint_y.as_ref()),
            Seed::from(&bump), // Reference to the local variable 'bump'
        ];
        let signer = Signer::from(&config_seeds);

        MintTo {
            mint: accounts.mint_lp,
            account: accounts.user_lp_ata,
            mint_authority: accounts.config,
            amount: data.amount,
        }
        .invoke_signed(&[signer])?;

        Ok(())
    }
}