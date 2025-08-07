use cosmwasm_std::Binary;
use cw_storage_plus::Item;

pub const DOMAIN_VK: Item<Binary> = Item::new("domain_vk");
