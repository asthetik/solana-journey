use std::mem::MaybeUninit;

use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{instructions::InitializeMint2, state::Mint};

use crate::Config;

/// 初始化 Config 账户，并存储 AMM 正常运行所需的所有信息。
/// 创建 mint_lp 铸币账户，并将 mint_authority 分配给 config 账户。
pub struct InitializeAccounts<'a> {
    pub initializer: &'a AccountView,
    pub mint_lp: &'a AccountView,
    pub config: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for InitializeAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let mut accounts_iter = accounts.iter();

        let initializer = accounts_iter
            .next()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let mint_lp = accounts_iter
            .next()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let config = accounts_iter
            .next()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;

        Ok(Self {
            initializer,
            mint_lp,
            config,
        })
    }
}

#[repr(C, packed)]
pub struct InitializeInstructionData {
    pub seed: u64,
    pub fee: u16,
    pub mint_x: [u8; 32],
    pub mint_y: [u8; 32],
    pub config_bump: [u8; 1],
    pub lp_bump: [u8; 1],
    pub authority: [u8; 32],
}

impl TryFrom<&[u8]> for InitializeInstructionData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        const INITIALIZE_DATA_LEN_WITH_AUTHORITY: usize = size_of::<InitializeInstructionData>();
        const INITIALIZE_DATA_LEN: usize =
            INITIALIZE_DATA_LEN_WITH_AUTHORITY - size_of::<[u8; 32]>();

        match data.len() {
            INITIALIZE_DATA_LEN_WITH_AUTHORITY => {
                Ok(unsafe { (data.as_ptr() as *const Self).read_unaligned() })
            }
            INITIALIZE_DATA_LEN => {
                // If the authority is not present, we need to build the buffer and add it at the end before transmuting to the struct
                let mut raw: MaybeUninit<[u8; INITIALIZE_DATA_LEN_WITH_AUTHORITY]> =
                    MaybeUninit::uninit();
                let raw_ptr = raw.as_mut_ptr() as *mut u8;
                unsafe {
                    // Copy the provided data
                    core::ptr::copy_nonoverlapping(data.as_ptr(), raw_ptr, INITIALIZE_DATA_LEN);
                    // Add the authority to the end of the buffer
                    core::ptr::write_bytes(raw_ptr.add(INITIALIZE_DATA_LEN), 0, 32);
                    // Now transmute to the struct
                    Ok((raw.as_ptr() as *const Self).read_unaligned())
                }
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

pub struct Initialize<'a> {
    pub accounts: InitializeAccounts<'a>,
    pub instruction_data: InitializeInstructionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Initialize<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let accounts = InitializeAccounts::try_from(accounts)?;
        let instruction_data: InitializeInstructionData =
            InitializeInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> Initialize<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0;

    pub fn process(&mut self) -> ProgramResult {
        let instruction_data = &self.instruction_data;
        let accounts = &self.accounts;
        let rent = Rent::get()?;

        // --- 1. 创建 Config 账户 ---
        let config_lamports = rent.try_minimum_balance(Config::LEN)?; // 动态计算
        let seed_binding = instruction_data.seed.to_le_bytes();
        let config_seeds = [
            Seed::from(b"config"),
            Seed::from(seed_binding.as_ref()),
            Seed::from(instruction_data.mint_x.as_ref()),
            Seed::from(instruction_data.mint_y.as_ref()),
            Seed::from(&instruction_data.config_bump),
        ];
        let config_signer = Signer::from(&config_seeds);
        // 计算 Config 账户所需的租金空间 (使用我们在 state.rs 定义的 LEN)
        CreateAccount {
            from: accounts.initializer,
            to: accounts.config,
            lamports: config_lamports, // 实际开发中应根据 Rent 计算，这里简化
            space: Config::LEN as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[config_signer])?;

        // --- 2. 初始化 Config 数据 ---
        // 使用之前实现的 load_mut_unchecked 来获取可变引用
        let config_account = unsafe { Config::load_mut_unchecked(accounts.config)? };
        config_account.set_inner(
            instruction_data.seed,
            instruction_data.authority.into(), // 将 [u8;32] 转为 Pubkey
            instruction_data.mint_x.into(),
            instruction_data.mint_y.into(),
            instruction_data.fee,
            instruction_data.config_bump,
        )?;

        // --- 3. 创建 Mint LP 账户 ---
        let mint_space = size_of::<Mint>();
        let mint_lamports = rent.try_minimum_balance(mint_space)?;
        let mint_lp_seeds = [
            Seed::from(b"mint_lp"),
            Seed::from(accounts.config.address().as_ref()),
            Seed::from(&instruction_data.lp_bump),
        ];

        // Mint 账户固定大小为 82 字节
        CreateAccount {
            from: accounts.initializer,
            to: accounts.mint_lp,
            lamports: mint_lamports, // 同样应根据 Rent 计算
            space: mint_space as u64,
            owner: &pinocchio_token::ID, // 注意所有者是 Token Program
        }
        .invoke_signed(&[Signer::from(&mint_lp_seeds)])?;

        // --- 4. 初始化 Mint LP (设置 Mint Authority) ---
        InitializeMint2 {
            mint: accounts.mint_lp,
            decimals: 6,                               // 通常 LP 代币使用 6 位小数
            mint_authority: accounts.config.address(), // 权限交给 Config PDA
            freeze_authority: None,
        }
        .invoke()?;

        Ok(())
    }
}