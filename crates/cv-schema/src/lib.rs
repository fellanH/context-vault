pub mod vaults;

pub use vaults::{
    VaultEntry, VaultRegistry, registry_path, load_registry, save_registry,
    resolve_active_vault, list_vaults, create_vault, use_vault, remove_vault,
    ensure_default_vault_registered,
};

pub mod indexing;
