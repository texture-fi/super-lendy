use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;
use texture_common::account::{PodAccount, PodAccountError};

use crate::state::CURATOR_DISCRIMINATOR;

pub const CURATOR_NAME_MAX_LEN: usize = 128;

pub const CURATOR_LOGO_URL_MAX_LEN: usize = 128;
pub const CURATOR_WEBSITE_URL_MAX_LEN: usize = 128;

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Curator {
    pub discriminator: [u8; 8],
    pub version: u8,

    /// Authority who can change this Curator account
    pub owner: Pubkey,

    /// Authority who can create Pools, Reserves and configure them.
    pub pools_authority: Pubkey,

    /// Authority who can create Vaults and configure them.
    pub vaults_authority: Pubkey,

    /// This is main wallet address (SOL holding, system program owned) who allowed to claim
    /// Curator's performance fees. Also ATA accounts of this authority are used as fee receivers
    /// for borrow fees in all Reserves owned by that Curator.
    pub fees_authority: Pubkey,

    pub name: [u8; CURATOR_NAME_MAX_LEN],
    pub logo_url: [u8; CURATOR_LOGO_URL_MAX_LEN],
    pub website_url: [u8; CURATOR_WEBSITE_URL_MAX_LEN],
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy)]
pub struct CuratorParams {
    /// Owner of the Curator account. Can change the account.
    pub owner: Pubkey,
    /// Authority who can claim fees
    pub fees_authority: Pubkey,
    /// Authority who can create Pools, Reserves and configure them.
    pub pools_authority: Pubkey,
    /// Authority who can create Vaults and configure them.
    pub vaults_authority: Pubkey,
    pub name: [u8; CURATOR_NAME_MAX_LEN],
    pub logo_url: [u8; CURATOR_LOGO_URL_MAX_LEN],
    pub website_url: [u8; CURATOR_WEBSITE_URL_MAX_LEN],
}

impl PodAccount for Curator {
    const DISCRIMINATOR: &'static [u8] = CURATOR_DISCRIMINATOR;

    type Version = u8;

    const VERSION: Self::Version = 1;

    type InitParams = CuratorParams;

    type InitError = PodAccountError;

    fn discriminator(&self) -> &[u8] {
        &self.discriminator
    }

    fn version(&self) -> Self::Version {
        self.version
    }

    fn init_unckecked(&mut self, params: Self::InitParams) -> Result<(), Self::InitError> {
        let Self {
            discriminator,
            version,
            owner,
            pools_authority,
            vaults_authority,
            fees_authority,
            name,
            logo_url,
            website_url,
        } = self;

        *discriminator = *CURATOR_DISCRIMINATOR;
        *version = Self::VERSION;
        *owner = params.owner;
        *pools_authority = params.pools_authority;
        *vaults_authority = params.vaults_authority;
        *fees_authority = params.fees_authority;
        *name = params.name;
        *logo_url = params.logo_url;
        *website_url = params.website_url;

        Ok(())
    }
}
