#![forbid(unsafe_code)]

use core::str::FromStr;

use radroots_core::{
    RadrootsCoreCurrency, RadrootsCoreDecimal, RadrootsCoreMoney, RadrootsCoreQuantity,
    RadrootsCoreQuantityPrice, RadrootsCoreUnit,
};
use radroots_events::{
    RadrootsNostrEventPtr,
    listing::{
        RadrootsListing, RadrootsListingAvailability, RadrootsListingBin,
        RadrootsListingDeliveryMethod, RadrootsListingFarmRef, RadrootsListingLocation,
        RadrootsListingProduct, RadrootsListingStatus,
    },
};
use radroots_events_codec::listing::encode::to_wire_parts as listing_to_wire_parts;
use radroots_nostr::prelude::{
    RadrootsNostrFilter, RadrootsNostrKind, RadrootsNostrTimestamp, radroots_event_from_nostr,
    radroots_nostr_parse_pubkey,
};
use radroots_trade::listing::{
    dvm::TradeListingAddress,
    dvm::{
        TradeListingEnvelope, TradeListingMessagePayload, TradeListingMessageType,
        TradeListingValidateRequest,
    },
    dvm_kinds::TRADE_LISTING_DVM_KINDS,
    order::{TradeOrder, TradeOrderItem, TradeOrderStatus},
    tags::trade_listing_dvm_tags,
    validation::{RadrootsTradeListing, validate_listing_event},
};

use super::RadrootsRuntime;
use crate::RadrootsAppError;

const LISTING_KIND: u32 = 30402;

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
    pub fn trade_listing_publish(
        &self,
        draft: TradeListingDraft,
    ) -> Result<String, RadrootsAppError> {
        let guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let mgr = guard
                .nostr
                .as_ref()
                .ok_or_else(|| RadrootsAppError::Msg("nostr not initialized".into()))?;
            let listing = listing_from_draft(&draft)?;
            let current_pubkey = current_pubkey_hex(self)?;
            if listing.farm.pubkey != current_pubkey {
                return Err(RadrootsAppError::Msg(
                    "farm_pubkey must match the active key".into(),
                ));
            }
            let parts = listing_to_wire_parts(&listing)
                .map_err(|e| RadrootsAppError::Msg(format!("listing encode failed: {e}")))?;
            mgr.send_custom_event_blocking(parts.kind, parts.content, parts.tags)
                .map_err(|e| RadrootsAppError::Msg(e.to_string()))
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn trade_listings_fetch(
        &self,
        limit: u16,
        since_unix: Option<u64>,
    ) -> Result<Vec<TradeListingSummary>, RadrootsAppError> {
        let guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let mgr = guard
                .nostr
                .as_ref()
                .ok_or_else(|| RadrootsAppError::Msg("nostr not initialized".into()))?;
            let mut filter =
                RadrootsNostrFilter::new().kind(RadrootsNostrKind::Custom(LISTING_KIND as u16));
            filter = filter.limit(limit.into());
            if let Some(since) = since_unix {
                filter = filter.since(RadrootsNostrTimestamp::from(since));
            }

            let events = mgr
                .fetch_events_blocking(filter, core::time::Duration::from_secs(10))
                .map_err(|e| RadrootsAppError::Msg(e.to_string()))?;
            let mut out = Vec::new();
            for ev in events {
                let event = radroots_event_from_nostr(&ev);
                if event.kind != LISTING_KIND {
                    continue;
                }
                match validate_listing_event(&event) {
                    Ok(listing) => {
                        out.push(listing_summary_from_trade(listing, &event));
                    }
                    Err(_) => continue,
                }
            }
            out.sort_by(|a, b| b.published_at.cmp(&a.published_at));
            Ok(out)
        }
        #[cfg(not(feature = "nostr-client"))]
        {
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
        let listing_addr = listing_addr_from_parts(&seller_pubkey, &listing_id)?;
        let payload =
            TradeListingMessagePayload::ListingValidateRequest(TradeListingValidateRequest {
                listing_event: Some(RadrootsNostrEventPtr {
                    id: listing_event_id,
                    relays: None,
                }),
            });
        self.send_trade_listing_message(
            TradeListingMessageType::ListingValidateRequest,
            listing_addr,
            None,
            payload,
            recipient_pubkey,
        )
    }

    pub fn trade_listing_send_order_request(
        &self,
        draft: TradeOrderDraft,
    ) -> Result<TradeOrderSendResult, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let order_id = normalize_optional_id(draft.order_id);
            let order_id = order_id
                .unwrap_or_else(|| format!("order-{}", chrono::Utc::now().timestamp_millis()));
            let buyer_pubkey = current_pubkey_hex(self)?;
            let seller_pubkey = normalize_pubkey(&draft.seller_pubkey)?;

            let bin_id = draft.bin_id.trim();
            if bin_id.is_empty() {
                return Err(RadrootsAppError::Msg("bin_id is required".into()));
            }
            let bin_count = parse_u32(&draft.bin_count, "bin_count")?;
            if bin_count == 0 {
                return Err(RadrootsAppError::Msg("bin_count must be > 0".into()));
            }

            let item = TradeOrderItem {
                bin_id: bin_id.to_string(),
                bin_count,
            };

            let notes = draft
                .notes
                .as_deref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            let order = TradeOrder {
                order_id: order_id.clone(),
                listing_addr: draft.listing_addr.clone(),
                buyer_pubkey,
                seller_pubkey,
                items: vec![item],
                discounts: None,
                notes,
                status: TradeOrderStatus::Requested,
            };

            let payload = TradeListingMessagePayload::OrderRequest(order);
            let event_id = self.send_trade_listing_message(
                TradeListingMessageType::OrderRequest,
                draft.listing_addr,
                Some(order_id.clone()),
                payload,
                draft.recipient_pubkey,
            )?;

            Ok(TradeOrderSendResult { event_id, order_id })
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn trade_listing_fetch_messages(
        &self,
        listing_addr: String,
        order_id: Option<String>,
        limit: u16,
        since_unix: Option<u64>,
    ) -> Result<Vec<TradeListingMessageSummary>, RadrootsAppError> {
        let guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let mgr = guard
                .nostr
                .as_ref()
                .ok_or_else(|| RadrootsAppError::Msg("nostr not initialized".into()))?;

            let kinds: Vec<RadrootsNostrKind> = TRADE_LISTING_DVM_KINDS
                .iter()
                .map(|k| RadrootsNostrKind::Custom(*k))
                .collect();

            let mut filter = RadrootsNostrFilter::new().kinds(kinds);
            filter = filter.limit(limit.into());
            if let Some(since) = since_unix {
                filter = filter.since(RadrootsNostrTimestamp::from(since));
            }

            let events = mgr
                .fetch_events_blocking(filter, core::time::Duration::from_secs(10))
                .map_err(|e| RadrootsAppError::Msg(e.to_string()))?;

            let mut out = Vec::new();
            for ev in events {
                let content = ev.content.clone();
                let envelope: TradeListingEnvelope<TradeListingMessagePayload> =
                    match serde_json::from_str(&content) {
                        Ok(env) => env,
                        Err(_) => continue,
                    };
                if envelope.validate().is_err() {
                    continue;
                }
                if envelope.listing_addr != listing_addr {
                    continue;
                }
                if let Some(ref oid) = order_id {
                    if envelope.order_id.as_deref() != Some(oid) {
                        continue;
                    }
                }
                let kind_u32 = ev.kind.as_u16() as u32;
                if envelope.message_type.kind() as u32 != kind_u32 {
                    continue;
                }

                let summary = message_summary(&envelope.payload);
                out.push(TradeListingMessageSummary {
                    event_id: ev.id.to_string(),
                    author: ev.pubkey.to_string(),
                    published_at: ev.created_at.as_secs(),
                    kind: kind_u32,
                    message_type: message_type_label(envelope.message_type).to_string(),
                    listing_addr: envelope.listing_addr.clone(),
                    order_id: envelope.order_id.clone(),
                    summary,
                    payload_json: content,
                });
            }
            out.sort_by(|a, b| b.published_at.cmp(&a.published_at));
            Ok(out)
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }
}

impl RadrootsRuntime {
    fn send_trade_listing_message(
        &self,
        message_type: TradeListingMessageType,
        listing_addr: String,
        order_id: Option<String>,
        payload: TradeListingMessagePayload,
        recipient_pubkey: String,
    ) -> Result<String, RadrootsAppError> {
        let guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let mgr = guard
                .nostr
                .as_ref()
                .ok_or_else(|| RadrootsAppError::Msg("nostr not initialized".into()))?;
            let recipient_hex = normalize_pubkey(&recipient_pubkey)?;
            let envelope = TradeListingEnvelope::new(
                message_type,
                listing_addr.clone(),
                order_id.clone(),
                payload,
            );
            envelope
                .validate()
                .map_err(|e| RadrootsAppError::Msg(e.to_string()))?;
            let content = serde_json::to_string(&envelope)
                .map_err(|e| RadrootsAppError::Msg(format!("encode envelope failed: {e}")))?;
            let tags = trade_listing_dvm_tags(recipient_hex, listing_addr, order_id);
            mgr.send_custom_event_blocking(message_type.kind() as u32, content, tags)
                .map_err(|e| RadrootsAppError::Msg(e.to_string()))
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }
}

fn listing_from_draft(draft: &TradeListingDraft) -> Result<RadrootsListing, RadrootsAppError> {
    let listing_id = draft
        .listing_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("listing-{}", chrono::Utc::now().timestamp_millis()));
    let farm_pubkey = draft.farm_pubkey.trim();
    if farm_pubkey.is_empty() {
        return Err(RadrootsAppError::Msg("farm_pubkey is required".into()));
    }
    let farm_pubkey = normalize_pubkey(farm_pubkey)?;
    let farm_d_tag = draft.farm_d_tag.trim();
    if farm_d_tag.is_empty() {
        return Err(RadrootsAppError::Msg("farm_d_tag is required".into()));
    }

    let title = draft.title.trim();
    if title.is_empty() {
        return Err(RadrootsAppError::Msg("title is required".into()));
    }
    let description = draft.description.trim();
    if description.is_empty() {
        return Err(RadrootsAppError::Msg("description is required".into()));
    }
    let category = draft.category.trim();
    if category.is_empty() {
        return Err(RadrootsAppError::Msg("category is required".into()));
    }
    let location_primary = draft.location_primary.trim();
    if location_primary.is_empty() {
        return Err(RadrootsAppError::Msg("location is required".into()));
    }

    let display_amount = parse_decimal(&draft.bin_display_amount, "bin_display_amount")?;
    ensure_non_negative(&display_amount, "bin_display_amount")?;
    let display_unit = parse_unit(&draft.bin_display_unit)?;
    let unit_price_amount = parse_decimal(&draft.unit_price, "unit_price")?;
    ensure_non_negative(&unit_price_amount, "unit_price")?;
    let currency = parse_currency(&draft.currency)?;
    let inventory = parse_decimal(&draft.inventory, "inventory")?;
    ensure_non_negative(&inventory, "inventory")?;

    let display_quantity = RadrootsCoreQuantity::new(display_amount, display_unit);
    let canonical_quantity = display_quantity
        .to_canonical()
        .map_err(|e| RadrootsAppError::Msg(format!("invalid bin_display_unit: {e}")))?;
    let unit_price = RadrootsCoreMoney::new(unit_price_amount, currency);
    let price_per_display_unit = RadrootsCoreQuantityPrice::new(
        unit_price.clone(),
        RadrootsCoreQuantity::new(RadrootsCoreDecimal::ONE, display_unit),
    );
    let price_per_canonical_unit = price_per_display_unit
        .try_to_canonical_unit_price()
        .map_err(|e| RadrootsAppError::Msg(format!("invalid unit_price: {e:?}")))?;
    let bin_label = clean_optional(&draft.bin_label);
    let bin_id = normalize_optional_id(draft.bin_id.clone()).unwrap_or_else(|| "bin-1".to_string());
    let bin = RadrootsListingBin {
        bin_id: bin_id.clone(),
        quantity: canonical_quantity,
        price_per_canonical_unit,
        display_amount: Some(display_amount),
        display_unit: Some(display_unit),
        display_label: bin_label,
        display_price: Some(unit_price),
        display_price_unit: Some(display_unit),
    };

    let delivery_method = parse_delivery_method(&draft.delivery_method)?;

    Ok(RadrootsListing {
        d_tag: listing_id,
        farm: RadrootsListingFarmRef {
            pubkey: farm_pubkey,
            d_tag: farm_d_tag.to_string(),
        },
        product: RadrootsListingProduct {
            key: category.to_string(),
            title: title.to_string(),
            category: category.to_string(),
            summary: Some(description.to_string()),
            process: None,
            lot: None,
            location: None,
            profile: None,
            year: None,
        },
        primary_bin_id: bin_id,
        bins: vec![bin],
        resource_area: None,
        plot: None,
        discounts: None,
        inventory_available: Some(inventory),
        availability: Some(RadrootsListingAvailability::Status {
            status: RadrootsListingStatus::Active,
        }),
        delivery_method: Some(delivery_method),
        location: Some(RadrootsListingLocation {
            primary: location_primary.to_string(),
            city: clean_optional(&draft.location_city),
            region: clean_optional(&draft.location_region),
            country: clean_optional(&draft.location_country),
            lat: None,
            lng: None,
            geohash: None,
        }),
        images: None,
    })
}

fn listing_summary_from_trade(
    listing: RadrootsTradeListing,
    event: &radroots_events::RadrootsNostrEvent,
) -> TradeListingSummary {
    let bin = listing
        .listing
        .bins
        .iter()
        .find(|bin| bin.bin_id == listing.primary_bin_id)
        .or_else(|| listing.listing.bins.first())
        .expect("validated listing must include bins");
    let (display_amount, display_unit) = match (bin.display_amount.as_ref(), bin.display_unit) {
        (Some(amount), Some(unit)) => (amount.clone(), unit),
        _ => (bin.quantity.amount.clone(), bin.quantity.unit),
    };
    let display_label = bin.display_label.clone().or(bin.quantity.label.clone());
    let display_label = clean_optional(&display_label);
    let (unit_price_amount, unit_price_currency, unit_price_unit) =
        match bin.price_per_canonical_unit.try_to_unit_price(display_unit) {
            Ok(price) => (
                price.amount.amount.to_string(),
                price.amount.currency.to_string(),
                price.quantity.unit.to_string(),
            ),
            Err(_) => match (&bin.display_price, bin.display_price_unit) {
                (Some(price), Some(unit)) => (
                    price.amount.to_string(),
                    price.currency.to_string(),
                    unit.to_string(),
                ),
                _ => (
                    bin.price_per_canonical_unit.amount.amount.to_string(),
                    bin.price_per_canonical_unit.amount.currency.to_string(),
                    bin.price_per_canonical_unit.quantity.unit.to_string(),
                ),
            },
        };

    TradeListingSummary {
        event_id: event.id.clone(),
        seller_pubkey: event.author.clone(),
        published_at: event.created_at as u64,
        listing_id: listing.listing_id,
        listing_addr: listing.listing_addr,
        title: listing.title,
        description: listing.description,
        product_type: listing.product_type,
        primary_bin_id: listing.primary_bin_id,
        unit_price_amount,
        unit_price_currency,
        unit_price_unit,
        bin_display_amount: display_amount.to_string(),
        bin_display_unit: display_unit.to_string(),
        bin_display_label: display_label,
        inventory_available: listing.inventory_available.to_string(),
        availability: availability_label(&listing.availability),
        location: format_location(&listing.location),
        delivery_method: delivery_method_label(&listing.delivery_method).to_string(),
    }
}

fn listing_addr_from_parts(
    seller_pubkey: &str,
    listing_id: &str,
) -> Result<String, RadrootsAppError> {
    let listing_id = listing_id.trim();
    if listing_id.is_empty() {
        return Err(RadrootsAppError::Msg("listing_id is required".into()));
    }
    let seller_hex = normalize_pubkey(seller_pubkey)?;
    Ok(TradeListingAddress {
        kind: LISTING_KIND as u16,
        seller_pubkey: seller_hex,
        listing_id: listing_id.to_string(),
    }
    .as_str())
}

fn normalize_pubkey(pubkey: &str) -> Result<String, RadrootsAppError> {
    let key = radroots_nostr_parse_pubkey(pubkey.trim())
        .map_err(|e| RadrootsAppError::Msg(e.to_string()))?;
    Ok(key.to_hex())
}

fn normalize_optional_id(id: Option<String>) -> Option<String> {
    id.as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[cfg(feature = "nostr-client")]
fn current_pubkey_hex(runtime: &RadrootsRuntime) -> Result<String, RadrootsAppError> {
    let guard = runtime
        .net
        .lock()
        .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
    let keys = guard
        .selected_nostr_keys()
        .ok_or_else(|| RadrootsAppError::Msg("no selected signing identity".into()))?;
    Ok(keys.public_key().to_hex())
}

fn parse_decimal(value: &str, label: &str) -> Result<RadrootsCoreDecimal, RadrootsAppError> {
    RadrootsCoreDecimal::from_str(value.trim())
        .map_err(|e| RadrootsAppError::Msg(format!("invalid {label}: {e}")))
}

fn parse_unit(value: &str) -> Result<RadrootsCoreUnit, RadrootsAppError> {
    RadrootsCoreUnit::from_str(value.trim())
        .map_err(|e| RadrootsAppError::Msg(format!("invalid unit: {e}")))
}

fn parse_currency(value: &str) -> Result<RadrootsCoreCurrency, RadrootsAppError> {
    RadrootsCoreCurrency::from_str(value.trim())
        .map_err(|e| RadrootsAppError::Msg(format!("invalid currency: {e}")))
}

fn parse_u32(value: &str, label: &str) -> Result<u32, RadrootsAppError> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|e| RadrootsAppError::Msg(format!("invalid {label}: {e}")))
}

fn ensure_non_negative(value: &RadrootsCoreDecimal, label: &str) -> Result<(), RadrootsAppError> {
    if value.is_sign_negative() {
        return Err(RadrootsAppError::Msg(format!(
            "{label} must be non-negative"
        )));
    }
    Ok(())
}

fn parse_delivery_method(value: &str) -> Result<RadrootsListingDeliveryMethod, RadrootsAppError> {
    let raw = value.trim();
    if raw.is_empty() {
        return Err(RadrootsAppError::Msg("delivery_method is required".into()));
    }
    let lowered = raw.to_ascii_lowercase();
    Ok(match lowered.as_str() {
        "pickup" => RadrootsListingDeliveryMethod::Pickup,
        "local_delivery" | "local delivery" => RadrootsListingDeliveryMethod::LocalDelivery,
        "shipping" => RadrootsListingDeliveryMethod::Shipping,
        _ => RadrootsListingDeliveryMethod::Other {
            method: raw.to_string(),
        },
    })
}

fn availability_label(availability: &RadrootsListingAvailability) -> String {
    match availability {
        RadrootsListingAvailability::Status { status } => match status {
            RadrootsListingStatus::Active => "active".to_string(),
            RadrootsListingStatus::Sold => "sold".to_string(),
            RadrootsListingStatus::Other { value } => value.clone(),
        },
        RadrootsListingAvailability::Window { start, end } => {
            let start = start
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".into());
            let end = end
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".into());
            format!("{start} - {end}")
        }
    }
}

fn delivery_method_label(method: &RadrootsListingDeliveryMethod) -> &'static str {
    match method {
        RadrootsListingDeliveryMethod::Pickup => "pickup",
        RadrootsListingDeliveryMethod::LocalDelivery => "local delivery",
        RadrootsListingDeliveryMethod::Shipping => "shipping",
        RadrootsListingDeliveryMethod::Other { .. } => "other",
    }
}

fn format_location(location: &RadrootsListingLocation) -> String {
    let mut parts = Vec::with_capacity(4);
    if !location.primary.trim().is_empty() {
        parts.push(location.primary.trim());
    }
    if let Some(city) = location.city.as_deref() {
        if !city.trim().is_empty() {
            parts.push(city.trim());
        }
    }
    if let Some(region) = location.region.as_deref() {
        if !region.trim().is_empty() {
            parts.push(region.trim());
        }
    }
    if let Some(country) = location.country.as_deref() {
        if !country.trim().is_empty() {
            parts.push(country.trim());
        }
    }
    if parts.is_empty() {
        "n/a".to_string()
    } else {
        parts.join(", ")
    }
}

fn clean_optional(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn message_type_label(message_type: TradeListingMessageType) -> &'static str {
    match message_type {
        TradeListingMessageType::ListingValidateRequest => "listing_validate_request",
        TradeListingMessageType::ListingValidateResult => "listing_validate_result",
        TradeListingMessageType::OrderRequest => "order_request",
        TradeListingMessageType::OrderResponse => "order_response",
        TradeListingMessageType::OrderRevision => "order_revision",
        TradeListingMessageType::OrderRevisionAccept => "order_revision_accept",
        TradeListingMessageType::OrderRevisionDecline => "order_revision_decline",
        TradeListingMessageType::Question => "question",
        TradeListingMessageType::Answer => "answer",
        TradeListingMessageType::DiscountRequest => "discount_request",
        TradeListingMessageType::DiscountOffer => "discount_offer",
        TradeListingMessageType::DiscountAccept => "discount_accept",
        TradeListingMessageType::DiscountDecline => "discount_decline",
        TradeListingMessageType::Cancel => "cancel",
        TradeListingMessageType::FulfillmentUpdate => "fulfillment_update",
        TradeListingMessageType::Receipt => "receipt",
    }
}

fn message_summary(payload: &TradeListingMessagePayload) -> String {
    match payload {
        TradeListingMessagePayload::ListingValidateRequest(_) => {
            "Listing validation requested".to_string()
        }
        TradeListingMessagePayload::ListingValidateResult(result) => {
            if result.valid {
                "Listing validated".to_string()
            } else if let Some(first) = result.errors.first() {
                format!("Listing invalid: {first}")
            } else {
                "Listing invalid".to_string()
            }
        }
        TradeListingMessagePayload::OrderRequest(order) => {
            let item = order.items.first();
            match item {
                Some(i) => format!("Order requested: {}x {}", i.bin_count, i.bin_id),
                None => "Order requested".to_string(),
            }
        }
        TradeListingMessagePayload::OrderResponse(res) => {
            if res.accepted {
                "Order accepted".to_string()
            } else if let Some(reason) = res.reason.as_deref() {
                format!("Order declined: {reason}")
            } else {
                "Order declined".to_string()
            }
        }
        TradeListingMessagePayload::OrderRevision(_) => "Order revision proposed".to_string(),
        TradeListingMessagePayload::OrderRevisionAccept(_) => "Order revision accepted".to_string(),
        TradeListingMessagePayload::OrderRevisionDecline(_) => {
            "Order revision declined".to_string()
        }
        TradeListingMessagePayload::Question(q) => format!("Question: {}", q.question_text),
        TradeListingMessagePayload::Answer(a) => format!("Answer: {}", a.answer_text),
        TradeListingMessagePayload::DiscountRequest(_) => "Discount requested".to_string(),
        TradeListingMessagePayload::DiscountOffer(_) => "Discount offered".to_string(),
        TradeListingMessagePayload::DiscountAccept(_) => "Discount accepted".to_string(),
        TradeListingMessagePayload::DiscountDecline(_) => "Discount declined".to_string(),
        TradeListingMessagePayload::Cancel(c) => {
            if let Some(reason) = c.reason.as_deref() {
                format!("Order cancelled: {reason}")
            } else {
                "Order cancelled".to_string()
            }
        }
        TradeListingMessagePayload::FulfillmentUpdate(update) => {
            format!("Fulfillment update: {:?}", update.status)
        }
        TradeListingMessagePayload::Receipt(receipt) => {
            if receipt.acknowledged {
                "Receipt acknowledged".to_string()
            } else {
                "Receipt update".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_from_draft_requires_fields() {
        let draft = TradeListingDraft {
            listing_id: None,
            farm_pubkey: "".into(),
            farm_d_tag: "".into(),
            title: "".into(),
            description: "Desc".into(),
            category: "Coffee".into(),
            bin_display_amount: "1".into(),
            bin_display_unit: "lb".into(),
            unit_price: "10.00".into(),
            currency: "USD".into(),
            bin_label: None,
            bin_id: None,
            inventory: "10".into(),
            delivery_method: "shipping".into(),
            location_primary: "Farm".into(),
            location_city: None,
            location_region: None,
            location_country: None,
        };
        assert!(listing_from_draft(&draft).is_err());
    }

    #[test]
    fn listing_from_draft_builds_listing() {
        let draft = TradeListingDraft {
            listing_id: Some("AAAAAAAAAAAAAAAAAAAAAg".into()),
            farm_pubkey: "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6".into(),
            farm_d_tag: "AAAAAAAAAAAAAAAAAAAAAA".into(),
            title: "Coffee".into(),
            description: "Washed".into(),
            category: "coffee".into(),
            bin_display_amount: "1".into(),
            bin_display_unit: "lb".into(),
            unit_price: "12.50".into(),
            currency: "USD".into(),
            bin_label: Some("bag".into()),
            bin_id: None,
            inventory: "5".into(),
            delivery_method: "shipping".into(),
            location_primary: "Farm".into(),
            location_city: Some("Town".into()),
            location_region: Some("Region".into()),
            location_country: Some("US".into()),
        };
        let listing = listing_from_draft(&draft).expect("listing builds");
        assert_eq!(listing.d_tag, "AAAAAAAAAAAAAAAAAAAAAg");
        assert_eq!(listing.product.title, "Coffee");
        assert!(listing.delivery_method.is_some());
        assert!(listing.location.is_some());
    }
}
