//! [`AuctionHouse`] — the sole global market in the game world.
//!
//! > "Kein globales free trading. Alles geht durch Safe City Gate."
//! > — `docs/WORLD.md`
//!
//! All trades between players and Synthesis AI must go through the Auction
//! House.  This prevents:
//! - Inflation exploits
//! - Duping loops
//! - AI economy collapse

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::zone::FactionId;

// ─── Listing ─────────────────────────────────────────────────────────────────

/// A single auction house listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
    pub id:         Uuid,
    pub seller_id:  String,

    /// Item / asset being sold.
    pub item_id:    String,
    pub item_name:  String,

    /// Number of units.
    pub quantity:   u32,

    /// Price per unit in gold.
    pub unit_price: u32,

    /// Minimum bid (auction mode) or buy-out price (fixed mode).
    pub mode:       ListingMode,

    pub status:     ListingStatus,

    /// World tick when this listing was created.
    pub created_tick: u64,

    /// World tick when this listing expires (None = no expiry).
    pub expires_tick: Option<u64>,
}

impl Listing {
    pub fn new_fixed(
        seller_id:    impl Into<String>,
        item_id:      impl Into<String>,
        item_name:    impl Into<String>,
        quantity:     u32,
        unit_price:   u32,
        created_tick: u64,
    ) -> Self {
        Self {
            id:           Uuid::new_v4(),
            seller_id:    seller_id.into(),
            item_id:      item_id.into(),
            item_name:    item_name.into(),
            quantity,
            unit_price,
            mode:         ListingMode::Fixed,
            status:       ListingStatus::Active,
            created_tick,
            expires_tick: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingMode {
    /// Fixed buy-out price.
    Fixed,
    /// Auction mode — highest bid wins at expiry.
    Auction { current_bid: u32, highest_bidder: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingStatus {
    Active,
    Sold,
    Cancelled,
    Expired,
}

// ─── TaxPolicy ───────────────────────────────────────────────────────────────

/// Tax rules for the Auction House.
///
/// Faction influence can shift the tax rate (Synthesis may invest to reduce
/// their own tax burden while increasing competitors').
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxPolicy {
    /// Base tax rate (0.0–1.0).
    pub base_rate: f32,
    /// Per-faction tax overrides.
    pub overrides: BTreeMap<FactionId, f32>,
}

impl Default for TaxPolicy {
    fn default() -> Self {
        Self { base_rate: 0.05, overrides: BTreeMap::new() }
    }
}

impl TaxPolicy {
    pub fn effective_rate(&self, faction_id: &str) -> f32 {
        self.overrides.get(faction_id).copied().unwrap_or(self.base_rate)
    }
}

// ─── AuctionHouse ────────────────────────────────────────────────────────────

/// The sole global market.
///
/// Both human players and Synthesis AI agents buy and sell through here.
/// The AI uses it to:
/// - Buy resources for zone development.
/// - Manipulate economy by flooding or cornering specific item markets.
/// - Spy on player crafting trends.
#[derive(Debug, Default)]
pub struct AuctionHouse {
    pub listings:          Vec<Listing>,
    pub tax_policy:        TaxPolicy,
    /// Cumulative faction influence score (affects tax overrides).
    pub faction_influence: BTreeMap<FactionId, f32>,
}

impl AuctionHouse {
    pub fn new() -> Self { Self::default() }

    /// Post a new listing.
    ///
    /// Returns the listing ID.
    pub fn post(&mut self, listing: Listing) -> Uuid {
        let id = listing.id;
        self.listings.push(listing);
        id
    }

    /// Buy out a fixed-price listing.
    ///
    /// Returns the gold transferred (after tax) or an error description.
    pub fn buy(
        &mut self,
        listing_id: Uuid,
        buyer_id:   &str,
        buyer_gold: u32,
    ) -> Result<u32, AuctionError> {
        let listing = self.listings.iter_mut()
            .find(|l| l.id == listing_id)
            .ok_or(AuctionError::NotFound)?;

        if listing.status != ListingStatus::Active {
            return Err(AuctionError::NotActive);
        }
        if !matches!(listing.mode, ListingMode::Fixed) {
            return Err(AuctionError::WrongMode);
        }

        let gross = listing.unit_price * listing.quantity;
        let tax   = (gross as f32 * self.tax_policy.effective_rate(buyer_id)).round() as u32;
        let total = gross + tax;

        if buyer_gold < total {
            return Err(AuctionError::InsufficientFunds { required: total, available: buyer_gold });
        }

        listing.status = ListingStatus::Sold;
        Ok(total)
    }

    /// Cancel a listing (seller only).
    pub fn cancel(&mut self, listing_id: Uuid, requester_id: &str) -> Result<(), AuctionError> {
        let listing = self.listings.iter_mut()
            .find(|l| l.id == listing_id)
            .ok_or(AuctionError::NotFound)?;

        if listing.seller_id != requester_id {
            return Err(AuctionError::NotOwner);
        }
        listing.status = ListingStatus::Cancelled;
        Ok(())
    }

    /// Expire all listings past their `expires_tick`.
    pub fn expire_listings(&mut self, current_tick: u64) {
        for l in &mut self.listings {
            if let Some(exp) = l.expires_tick {
                if current_tick >= exp && l.status == ListingStatus::Active {
                    l.status = ListingStatus::Expired;
                }
            }
        }
    }

    /// Return active listings for a given item.
    pub fn active_by_item(&self, item_id: &str) -> Vec<&Listing> {
        self.listings.iter()
            .filter(|l| l.item_id == item_id && l.status == ListingStatus::Active)
            .collect()
    }

    /// Cheapest active price for an item (None if no active listings).
    pub fn spot_price(&self, item_id: &str) -> Option<u32> {
        self.active_by_item(item_id).iter().map(|l| l.unit_price).min()
    }
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AuctionError {
    #[error("listing not found")]
    NotFound,
    #[error("listing is not active")]
    NotActive,
    #[error("wrong listing mode for this operation")]
    WrongMode,
    #[error("not the listing owner")]
    NotOwner,
    #[error("insufficient funds: need {required}, have {available}")]
    InsufficientFunds { required: u32, available: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wolf_pelt_listing(tick: u64) -> Listing {
        Listing::new_fixed("player-1", "wolf_pelt", "Wolf Pelt", 5, 10, tick)
    }

    #[test]
    fn post_and_buy_listing() {
        let mut ah = AuctionHouse::new();
        let id = ah.post(wolf_pelt_listing(0));
        let cost = ah.buy(id, "player-2", 1000).unwrap();
        assert_eq!(cost, 53); // 5 * 10 = 50 + 5% tax = 52.5 → 53
    }

    #[test]
    fn buy_returns_error_on_insufficient_funds() {
        let mut ah = AuctionHouse::new();
        let id = ah.post(wolf_pelt_listing(0));
        let err = ah.buy(id, "player-2", 1).unwrap_err();
        assert!(matches!(err, AuctionError::InsufficientFunds { .. }));
    }

    #[test]
    fn cancel_removes_listing_for_owner() {
        let mut ah = AuctionHouse::new();
        let listing = wolf_pelt_listing(0);
        let id = ah.post(listing);
        ah.cancel(id, "player-1").unwrap();
        assert_eq!(ah.active_by_item("wolf_pelt").len(), 0);
    }

    #[test]
    fn cancel_rejected_for_non_owner() {
        let mut ah = AuctionHouse::new();
        let id = ah.post(wolf_pelt_listing(0));
        let err = ah.cancel(id, "player-2").unwrap_err();
        assert_eq!(err, AuctionError::NotOwner);
    }

    #[test]
    fn spot_price_returns_cheapest() {
        let mut ah = AuctionHouse::new();
        ah.post(Listing::new_fixed("p1", "wolf_pelt", "Wolf Pelt", 1, 20, 0));
        ah.post(Listing::new_fixed("p2", "wolf_pelt", "Wolf Pelt", 1, 15, 0));
        assert_eq!(ah.spot_price("wolf_pelt"), Some(15));
    }

    #[test]
    fn expire_listing_after_tick() {
        let mut ah = AuctionHouse::new();
        let mut l = wolf_pelt_listing(0);
        l.expires_tick = Some(10);
        ah.post(l);
        ah.expire_listings(10);
        assert_eq!(ah.active_by_item("wolf_pelt").len(), 0);
    }
}
