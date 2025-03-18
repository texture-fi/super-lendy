use crate::instruction::{AlterCuratorAccounts, CreateCuratorAccounts};
use crate::processor::Processor;
use crate::state::curator::{Curator, CuratorParams};
use crate::state::texture_cfg::TextureConfig;
use crate::LendyResult;
use solana_program::msg;
use texture_common::account::PodAccount;
use texture_common::utils::verify_key;

impl<'a, 'b> Processor<'a, 'b> {
    #[inline(never)]
    pub(super) fn create_curator(&self, params: CuratorParams) -> LendyResult<()> {
        msg!("create_curator ix: {:?}", params);

        let CreateCuratorAccounts {
            curator,
            texture_config,
            global_config_owner,
        } = CreateCuratorAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        let cfg_data = texture_config.data.borrow();
        let unpacked_cfg = TextureConfig::try_from_bytes(cfg_data.as_ref())?;

        // Only global config owner can create new Curator accounts.
        verify_key(
            global_config_owner.key,
            &unpacked_cfg.owner,
            "global config owner",
        )?;

        // Initialize internal structure of the curator account.
        // Account itself must be already created (rent exempt) and assigned to Super Lendy
        let mut curator_data = curator.data.borrow_mut();
        Curator::init_bytes(curator_data.as_mut(), params)?;

        Ok(())
    }

    #[inline(never)]
    pub(super) fn alter_curator(&self, params: CuratorParams) -> LendyResult<()> {
        msg!("alter_curator ix: {:?}", params);

        let AlterCuratorAccounts { curator, owner } =
            AlterCuratorAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        let mut curator_data = curator.data.borrow_mut();
        let unpacked_curator = Curator::try_from_bytes_mut(curator_data.as_mut())?;

        verify_key(owner.key, &unpacked_curator.owner, "owner")?;

        unpacked_curator.owner = params.owner;
        unpacked_curator.name = params.name;
        unpacked_curator.logo_url = params.logo_url;
        unpacked_curator.website_url = params.website_url;
        unpacked_curator.fees_authority = params.fees_authority;
        unpacked_curator.pools_authority = params.pools_authority;
        unpacked_curator.vaults_authority = params.vaults_authority;

        Ok(())
    }
}
