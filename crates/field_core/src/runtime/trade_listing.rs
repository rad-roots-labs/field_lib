#![forbid(unsafe_code)]

use core::str::FromStr;

use radroots_core::{
    RadrootsCoreCurrency, RadrootsCoreDecimal, RadrootsCoreMoney, RadrootsCoreQuantity,
    RadrootsCoreQuantityPrice, RadrootsCoreUnit,
};
use radroots_events::{
    RadrootsNostrEvent,
    farm::RadrootsFarmRef,
    ids::{RadrootsDTag, RadrootsInventoryBinId},
    kinds::KIND_LISTING,
    listing::{
        RadrootsListing, RadrootsListingAvailability, RadrootsListingBin,
        RadrootsListingDeliveryMethod, RadrootsListingLocation, RadrootsListingProduct,
        RadrootsListingStatus,
    },
};
use radroots_events_codec::listing::encode::to_wire_parts as listing_to_wire_parts;
use radroots_nostr::prelude::{
    RadrootsNostrFilter, RadrootsNostrKind, RadrootsNostrTimestamp, radroots_event_from_nostr,
};
use radroots_trade::listing::validation::validate_listing_event;

use super::RadrootsRuntime;
use crate::RadrootsAppError;

#[derive(uniffi::Record, Debug, Clone)]
pub struct TradeListingDraft {
    pub listing_id: Option<String>,
    pub farm_pubkey: String,
    pub farm_d_tag: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub bin_display_amount: String,
    pub bin_display_unit: String,
    pub unit_price: String,
    pub currency: String,
    pub bin_label: Option<String>,
    pub bin_id: Option<String>,
    pub inventory: String,
    pub delivery_method: String,
    pub location_primary: String,
    pub location_city: Option<String>,
    pub location_region: Option<String>,
    pub location_country: Option<String>,
}

#[derive(uniffi::Record, Debug, Clone)]
pub struct TradeListingSummary {
    pub event_id: String,
    pub seller_pubkey: String,
    pub published_at: u64,
    pub listing_id: String,
    pub listing_addr: String,
    pub title: String,
    pub description: String,
    pub product_type: String,
    pub primary_bin_id: String,
    pub unit_price_amount: String,
    pub unit_price_currency: String,
    pub unit_price_unit: String,
    pub bin_display_amount: String,
    pub bin_display_unit: String,
    pub bin_display_label: Option<String>,
    pub inventory_available: String,
    pub availability: String,
    pub location: String,
    pub delivery_method: String,
}

#[derive(uniffi::Record, Debug, Clone)]
pub struct TradeListingEventParts {
    pub kind: u32,
    pub content: String,
    pub tags_json: String,
}

#[derive(uniffi::Record, Debug, Clone)]
pub struct TradeOrderDraft {
    pub listing_addr: String,
    pub seller_pubkey: String,
    pub bin_id: String,
    pub bin_count: String,
    pub notes: Option<String>,
    pub order_id: Option<String>,
    pub recipient_pubkey: String,
}

#[derive(uniffi::Record, Debug, Clone)]
pub struct TradeOrderSendResult {
    pub event_id: String,
    pub order_id: String,
}

#[derive(uniffi::Record, Debug, Clone)]
pub struct TradeListingMessageSummary {
    pub event_id: String,
    pub author: String,
    pub published_at: u64,
    pub kind: u32,
    pub message_type: String,
    pub listing_addr: String,
    pub order_id: Option<String>,
    pub summary: String,
    pub payload_json: String,
}

#[cfg_attr(not(coverage_nightly), uniffi::export)]
impl RadrootsRuntime {
    pub fn trade_listing_build_event_parts(
        &self,
        draft: TradeListingDraft,
    ) -> Result<TradeListingEventParts, RadrootsAppError> {
        let listing = listing_from_draft(&draft)?;
        let parts = listing_to_wire_parts(&listing)
            .map_err(|error| RadrootsAppError::Msg(format!("listing encode failed: {error}")))?;
        let tags_json = serde_json::to_string(&parts.tags).map_err(|error| {
            RadrootsAppError::Msg(format!("listing tags encode failed: {error}"))
        })?;
        Ok(TradeListingEventParts {
            kind: parts.kind,
            content: parts.content,
            tags_json,
        })
    }

    pub fn trade_listing_publish(
        &self,
        draft: TradeListingDraft,
    ) -> Result<String, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let guard = self
                .net
                .lock()
                .map_err(|error| RadrootsAppError::Msg(format!("{error}")))?;
            let mgr = guard
                .nostr
                .as_ref()
                .ok_or_else(|| RadrootsAppError::Msg("nostr not initialized".into()))?;
            let listing = listing_from_draft(&draft)?;
            let current_pubkey = current_pubkey_hex(self)?;
            if listing.farm.pubkey != current_pubkey {
                return Err(RadrootsAppError::Msg(
                    "farm_pubkey must match the default account public key".into(),
                ));
            }
            let parts = listing_to_wire_parts(&listing).map_err(|error| {
                RadrootsAppError::Msg(format!("listing encode failed: {error}"))
            })?;
            mgr.send_custom_event_blocking(parts.kind, parts.content, parts.tags)
                .map_err(|error| RadrootsAppError::Msg(error.to_string()))
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            let _ = draft;
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn trade_listings_fetch(
        &self,
        limit: u16,
        since_unix: Option<u64>,
    ) -> Result<Vec<TradeListingSummary>, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let guard = self
                .net
                .lock()
                .map_err(|error| RadrootsAppError::Msg(format!("{error}")))?;
            let mgr = guard
                .nostr
                .as_ref()
                .ok_or_else(|| RadrootsAppError::Msg("nostr not initialized".into()))?;
            let mut filter =
                RadrootsNostrFilter::new().kind(RadrootsNostrKind::Custom(KIND_LISTING as u16));
            filter = filter.limit(limit.into());
            if let Some(since) = since_unix {
                filter = filter.since(RadrootsNostrTimestamp::from(since));
            }

            let events = mgr
                .fetch_events_blocking(filter, core::time::Duration::from_secs(10))
                .map_err(|error| RadrootsAppError::Msg(error.to_string()))?;
            let mut out = Vec::new();
            for event in events {
                let event = radroots_event_from_nostr(&event);
                if let Ok(listing) = validate_listing_event(&event) {
                    out.push(listing_summary_from_trade(listing, &event));
                }
            }
            out.sort_by(|left, right| right.published_at.cmp(&left.published_at));
            Ok(out)
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            let _ = (limit, since_unix);
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn trade_listing_send_validation_request(
        &self,
        listing_event_id: String,
        seller_pubkey: String,
        listing_id: String,
        recipient_pubkey: String,
    ) -> Result<String, RadrootsAppError> {
        let _ = (
            listing_event_id,
            seller_pubkey,
            listing_id,
            recipient_pubkey,
        );
        Err(RadrootsAppError::Msg(
            "legacy listing validation requests are retired".into(),
        ))
    }

    pub fn trade_listing_send_order_request(
        &self,
        draft: TradeOrderDraft,
    ) -> Result<TradeOrderSendResult, RadrootsAppError> {
        let _ = draft;
        Err(RadrootsAppError::Msg(
            "legacy listing order requests are retired; use active trade order APIs".into(),
        ))
    }

    pub fn trade_listing_fetch_messages(
        &self,
        listing_addr: String,
        limit: u16,
        since_unix: Option<u64>,
    ) -> Result<Vec<TradeListingMessageSummary>, RadrootsAppError> {
        let _ = (listing_addr, limit, since_unix);
        Ok(Vec::new())
    }
}

fn listing_from_draft(draft: &TradeListingDraft) -> Result<RadrootsListing, RadrootsAppError> {
    let listing_id = non_empty(
        draft
            .listing_id
            .clone()
            .unwrap_or_else(|| format!("listing-{}", chrono::Utc::now().timestamp_millis())),
        "listing_id",
    )?;
    let farm_pubkey = non_empty(draft.farm_pubkey.clone(), "farm_pubkey")?;
    let farm_d_tag = non_empty(draft.farm_d_tag.clone(), "farm_d_tag")?;
    let title = non_empty(draft.title.clone(), "title")?;
    let description = non_empty(draft.description.clone(), "description")?;
    let category = non_empty(draft.category.clone(), "category")?;
    let bin_id = non_empty(
        draft.bin_id.clone().unwrap_or_else(|| "bin-1".to_string()),
        "bin_id",
    )?;
    let listing_id = RadrootsDTag::parse(listing_id)
        .map_err(|error| RadrootsAppError::Msg(format!("invalid listing_id: {error}")))?;
    let bin_id = RadrootsInventoryBinId::parse(bin_id)
        .map_err(|error| RadrootsAppError::Msg(format!("invalid bin_id: {error}")))?;
    let amount = parse_decimal(&draft.bin_display_amount, "bin_display_amount")?;
    let unit = parse_unit(&draft.bin_display_unit)?;
    let canonical_unit = unit.canonical_unit();
    let currency = parse_currency(&draft.currency)?;
    let unit_price = parse_decimal(&draft.unit_price, "unit_price")?;
    let inventory = parse_decimal(&draft.inventory, "inventory")?;
    let location_primary = non_empty(draft.location_primary.clone(), "location_primary")?;

    Ok(RadrootsListing {
        d_tag: listing_id,
        published_at: None,
        farm: RadrootsFarmRef {
            pubkey: farm_pubkey,
            d_tag: farm_d_tag,
        },
        product: RadrootsListingProduct {
            key: category.clone(),
            title,
            category,
            summary: Some(description),
            process: None,
            lot: None,
            location: None,
            profile: None,
            year: None,
        },
        primary_bin_id: bin_id.clone(),
        bins: vec![RadrootsListingBin {
            bin_id,
            quantity: RadrootsCoreQuantity::new(amount, canonical_unit),
            price_per_canonical_unit: RadrootsCoreQuantityPrice::new(
                RadrootsCoreMoney::new(unit_price, currency),
                RadrootsCoreQuantity::new(RadrootsCoreDecimal::ONE, canonical_unit),
            ),
            display_amount: Some(amount),
            display_unit: Some(unit),
            display_label: draft.bin_label.clone(),
            display_price: Some(RadrootsCoreMoney::new(unit_price, currency)),
            display_price_unit: Some(unit),
        }],
        resource_area: None,
        plot: None,
        discounts: None,
        inventory_available: Some(inventory),
        availability: Some(RadrootsListingAvailability::Status {
            status: RadrootsListingStatus::Active,
        }),
        delivery_method: Some(parse_delivery_method(&draft.delivery_method)),
        location: Some(RadrootsListingLocation {
            primary: location_primary,
            city: blank_to_none(draft.location_city.clone()),
            region: blank_to_none(draft.location_region.clone()),
            country: blank_to_none(draft.location_country.clone()),
            lat: None,
            lng: None,
            geohash: None,
        }),
        images: None,
    })
}

fn listing_summary_from_trade(
    listing: radroots_trade::listing::validation::RadrootsTradeListing,
    event: &RadrootsNostrEvent,
) -> TradeListingSummary {
    let primary_bin = listing
        .listing
        .bins
        .iter()
        .find(|bin| bin.bin_id == listing.primary_bin_id);
    TradeListingSummary {
        event_id: event.id.clone(),
        seller_pubkey: listing.seller_pubkey,
        published_at: event.created_at as u64,
        listing_id: listing.listing_id,
        listing_addr: listing.listing_addr,
        title: listing.title,
        description: listing.description,
        product_type: listing.product_type,
        primary_bin_id: listing.primary_bin_id,
        unit_price_amount: listing.unit_price.amount.to_string(),
        unit_price_currency: listing.unit_price.currency.to_string(),
        unit_price_unit: listing.unit.to_string(),
        bin_display_amount: primary_bin
            .and_then(|bin| bin.display_amount)
            .unwrap_or(listing.bin_quantity.amount)
            .to_string(),
        bin_display_unit: primary_bin
            .and_then(|bin| bin.display_unit)
            .unwrap_or(listing.unit)
            .to_string(),
        bin_display_label: primary_bin.and_then(|bin| bin.display_label.clone()),
        inventory_available: listing.inventory_available.to_string(),
        availability: availability_label(&listing.availability),
        location: location_label(&listing.location),
        delivery_method: delivery_method_label(&listing.delivery_method),
    }
}

#[cfg(feature = "nostr-client")]
fn current_pubkey_hex(runtime: &RadrootsRuntime) -> Result<String, RadrootsAppError> {
    let guard = runtime
        .net
        .lock()
        .map_err(|error| RadrootsAppError::Msg(format!("{error}")))?;
    let identity = guard
        .accounts
        .default_public_identity()
        .map_err(|error| RadrootsAppError::Msg(format!("{error}")))?
        .ok_or_else(|| RadrootsAppError::Msg("default account is not configured".into()))?;
    Ok(identity.public_key_hex)
}

fn non_empty(value: String, field: &str) -> Result<String, RadrootsAppError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(RadrootsAppError::Msg(format!("{field} is required")));
    }
    Ok(value)
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_decimal(value: &str, field: &str) -> Result<RadrootsCoreDecimal, RadrootsAppError> {
    RadrootsCoreDecimal::from_str(value.trim())
        .map_err(|error| RadrootsAppError::Msg(format!("{field} is invalid: {error}")))
}

fn parse_currency(value: &str) -> Result<RadrootsCoreCurrency, RadrootsAppError> {
    RadrootsCoreCurrency::from_str(value.trim())
        .map_err(|error| RadrootsAppError::Msg(format!("currency is invalid: {error}")))
}

fn parse_unit(value: &str) -> Result<RadrootsCoreUnit, RadrootsAppError> {
    RadrootsCoreUnit::from_str(value.trim())
        .map_err(|error| RadrootsAppError::Msg(format!("unit is invalid: {error}")))
}

fn parse_delivery_method(value: &str) -> RadrootsListingDeliveryMethod {
    match value.trim().to_ascii_lowercase().as_str() {
        "pickup" => RadrootsListingDeliveryMethod::Pickup,
        "local_delivery" | "local delivery" => RadrootsListingDeliveryMethod::LocalDelivery,
        "shipping" => RadrootsListingDeliveryMethod::Shipping,
        other => RadrootsListingDeliveryMethod::Other {
            method: other.to_string(),
        },
    }
}

fn availability_label(value: &RadrootsListingAvailability) -> String {
    match value {
        RadrootsListingAvailability::Window { start, end } => {
            format!("window:{start:?}:{end:?}")
        }
        RadrootsListingAvailability::Status { status } => match status {
            RadrootsListingStatus::Active => "active".to_string(),
            RadrootsListingStatus::Sold => "sold".to_string(),
            RadrootsListingStatus::Other { value } => value.clone(),
        },
    }
}

fn delivery_method_label(value: &RadrootsListingDeliveryMethod) -> String {
    match value {
        RadrootsListingDeliveryMethod::Pickup => "pickup".to_string(),
        RadrootsListingDeliveryMethod::LocalDelivery => "local_delivery".to_string(),
        RadrootsListingDeliveryMethod::Shipping => "shipping".to_string(),
        RadrootsListingDeliveryMethod::Other { method } => method.clone(),
    }
}

fn location_label(value: &RadrootsListingLocation) -> String {
    [
        Some(value.primary.as_str()),
        value.city.as_deref(),
        value.region.as_deref(),
        value.country.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|part| !part.trim().is_empty())
    .collect::<Vec<_>>()
    .join(", ")
}
