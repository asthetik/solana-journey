use core::mem::size_of;
use pinocchio::{
    AccountView, Address,
    account::{Ref, RefMut},
    error::ProgramError,
};

#[repr(C, packed)]
pub struct Config {
    state: u8,
    seed: [u8; 8],
    authority: Address,
    mint_x: Address,
    mint_y: Address,
    fee: [u8; 2],
    config_bump: [u8; 1],
}

#[repr(u8)]
pub enum AmmState {
    Uninitialized = 0u8,
    Initialized = 1u8,
    Disabled = 2u8,
    WithdrawOnly = 3u8,
}

impl Config {
    pub const LEN: usize = size_of::<Config>();

    #[inline(always)]
    pub fn load<'a>(account_view: &'a AccountView) -> Result<Ref<'a, Self>, ProgramError> {
        if account_view.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        // 在 Pinocchio 这种追求极限性能的底层框架中，unsafe 的存在是为了将控制权从编译器交还给开发者，以换取零成本抽象（Zero-cost Abstractions）。
        // Safety: We verify the owner before allowing access to the data
        let is_owner_valid = unsafe { account_view.owner() == &crate::ID };
        if !is_owner_valid {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let data = account_view.try_borrow()?;

        Ok(Ref::map(data, |data| unsafe {
            Self::from_bytes_unchecked(data)
        }))
    }

    #[inline(always)]
    /// # Safety
    /// This function is unsafe because it dereferences a raw pointer without checking nullability.
    pub unsafe fn load_unchecked(account_view: &AccountView) -> Result<&Self, ProgramError> {
        if account_view.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let is_owner_valid = unsafe { account_view.owner() == &crate::ID };
        if !is_owner_valid {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(unsafe { Self::from_bytes_unchecked(account_view.borrow_unchecked()) })
    }

    /// Return a `Config` from the given bytes.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `bytes` contains a valid representation of `Config`, and
    /// it is properly aligned to be interpreted as an instance of `Config`.
    /// At the moment `Config` has an alignment of 1 byte.
    /// This method does not perform a length validation.
    #[inline(always)]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes.as_ptr() as *const Config) }
    }

    /// Return a mutable `Config` reference from the given bytes.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `bytes` contains a valid representation of `Config`.
    #[inline(always)]
    pub unsafe fn from_bytes_unchecked_mut(bytes: &mut [u8]) -> &mut Self {
        unsafe { &mut *(bytes.as_mut_ptr() as *mut Config) }
    }

    // Getter methods for safe field access
    #[inline(always)]
    pub fn state(&self) -> u8 {
        self.state
    }

    #[inline(always)]
    pub fn seed(&self) -> u64 {
        u64::from_le_bytes(self.seed)
    }

    #[inline(always)]
    pub fn authority(&self) -> &Address {
        &self.authority
    }

    #[inline(always)]
    pub fn mint_x(&self) -> &Address {
        &self.mint_x
    }

    #[inline(always)]
    pub fn mint_y(&self) -> &Address {
        &self.mint_y
    }

    #[inline(always)]
    pub fn fee(&self) -> u16 {
        u16::from_le_bytes(self.fee)
    }

    #[inline(always)]
    pub fn config_bump(&self) -> [u8; 1] {
        self.config_bump
    }

    #[inline(always)]
    pub fn load_mut<'a>(account_view: &'a AccountView) -> Result<RefMut<'a, Self>, ProgramError> {
        if account_view.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let is_owner_valid = unsafe { account_view.owner() == &crate::ID };

        if !is_owner_valid {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(RefMut::map(account_view.try_borrow_mut()?, |data| unsafe {
            Self::from_bytes_unchecked_mut(data)
        }))
    }

    #[inline(always)]
    pub fn set_state(&mut self, state: u8) -> Result<(), ProgramError> {
        if state.ge(&(AmmState::WithdrawOnly as u8)) {
            return Err(ProgramError::InvalidAccountData);
        }
        self.state = state;
        Ok(())
    }

    #[inline(always)]
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed.to_le_bytes();
    }

    #[inline(always)]
    pub fn set_authority(&mut self, authority: Address) {
        self.authority = authority;
    }

    #[inline(always)]
    pub fn set_mint_x(&mut self, mint_x: Address) {
        self.mint_x = mint_x;
    }

    #[inline(always)]
    pub fn set_mint_y(&mut self, mint_y: Address) {
        self.mint_y = mint_y;
    }

    #[inline(always)]
    pub fn set_fee(&mut self, fee: u16) -> Result<(), ProgramError> {
        if fee.ge(&10_000) {
            return Err(ProgramError::InvalidAccountData);
        }
        self.fee = fee.to_le_bytes();
        Ok(())
    }

    #[inline(always)]
    pub fn set_config_bump(&mut self, config_bump: [u8; 1]) {
        self.config_bump = config_bump;
    }

    #[inline(always)]
    pub fn set_inner(
        &mut self,
        seed: u64,
        authority: Address,
        mint_x: Address,
        mint_y: Address,
        fee: u16,
        config_bump: [u8; 1],
    ) -> Result<(), ProgramError> {
        self.set_state(AmmState::Initialized as u8)?;
        self.set_seed(seed);
        self.set_authority(authority);
        self.set_mint_x(mint_x);
        self.set_mint_y(mint_y);
        self.set_fee(fee)?;
        self.set_config_bump(config_bump);
        Ok(())
    }

    #[inline(always)]
    pub fn has_authority(&self) -> Option<Address> {
        // read_unaligned：处理“对齐”问题
        // // 1. 用“不检查对齐”的方法，把 self.authority 里的内容强行拷贝一份给 auth
        // We use read_unaligned to safely copy the Address bytes into the 'auth' variable
        let auth = unsafe { core::ptr::addr_of!(self.authority).read_unaligned() };

        // 2. 检查复印出来的这个地址是不是全 0（默认地址）
        if auth == Address::default() {
            None // 全 0 说明没设置，返回“空”
        } else {
            // 3. 返回我们刚才“复印”好的 auth，而不是原件
            // 这样就不用从原结构体里“撕”数据了，编译器就不会报错
            // Return 'auth' (the local copy) instead of 'self.authority'
            Some(auth)
        }
    }

    /// 强制以可变引用加载账户数据，不检查所有者 (用于初始化)
    /// # Safety
    /// 调用者必须确保账户空间足够且已由程序控制
    #[allow(clippy::mut_from_ref)]
    #[inline(always)]
    pub unsafe fn load_mut_unchecked(
        account_view: &AccountView,
    ) -> Result<&mut Self, ProgramError> {
        if account_view.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        // 直接获取账户数据的原始指针并转换为可变结构体引用
        Ok(unsafe { Self::from_bytes_unchecked_mut(account_view.borrow_unchecked_mut()) })
    }
}