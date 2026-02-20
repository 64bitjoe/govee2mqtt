use crate::hass_mqtt::base::{Device, EntityConfig, Origin};
use crate::hass_mqtt::instance::{publish_entity_config, EntityInstance};
use crate::hass_mqtt::work_mode::ParsedWorkMode;
use crate::platform_api::{DeviceParameters, DeviceType, IntegerRange};
use crate::service::device::Device as ServiceDevice;
use crate::service::hass::{availability_topic, topic_safe_id, HassClient, IdParameter};
use crate::service::state::StateHandle;
use anyhow::anyhow;
use async_trait::async_trait;
use mosquitto_rs::router::{Params, Payload, State};
use serde::Serialize;

/// <https://www.home-assistant.io/integrations/fan.mqtt>
#[derive(Serialize, Clone, Debug)]
pub struct FanConfig {
    #[serde(flatten)]
    pub base: EntityConfig,

    pub command_topic: String,

    /// HASS will publish here to change the current preset mode
    pub mode_command_topic: String,
    /// we will publish the current preset mode here
    pub mode_state_topic: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_range_min: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_range_max: Option<u8>,
    /// HASS will publish here to change the current speed percentage
    pub percentage_command_topic: String,
    /// we will publish the current speed percentage here
    pub percentage_state_topic: String,

    pub optimistic: bool,

    /// The list of supported preset modes
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub preset_modes: Vec<String>,

    pub state_topic: String,
}

#[derive(Clone)]
pub struct Fan {
    fan: FanConfig,
    state: StateHandle,
    device_id: String,
}

impl Fan {
    pub async fn new(device: &ServiceDevice, state: &StateHandle) -> anyhow::Result<Self> {
        let use_iot = device.iot_api_supported() && state.get_iot_client().await.is_some();
        let optimistic = !use_iot;

        // command_topic controls the power state; route it to the general power switch handler
        let command_topic = format!(
            "gv2mqtt/switch/{id}/command/powerSwitch",
            id = topic_safe_id(device)
        );

        let state_topic = format!("gv2mqtt/fan/{id}/state", id = topic_safe_id(device));
        let mode_state_topic = format!(
            "gv2mqtt/fan/{id}/notify-mode",
            id = topic_safe_id(device)
        );
        let mode_command_topic = format!("gv2mqtt/fan/{id}/set-mode", id = topic_safe_id(device));
        let percentage_command_topic =
            format!("gv2mqtt/fan/{id}/set-speed", id = topic_safe_id(device));
        let percentage_state_topic = format!(
            "gv2mqtt/fan/{id}/notify-speed",
            id = topic_safe_id(device)
        );

        let unique_id = format!("gv2mqtt-{id}-fan", id = topic_safe_id(device));

        let mut speed_range_min = None;
        let mut speed_range_max = None;

        if let Some(info) = &device.http_device_info {
            if let Some(cap) = info.capability_by_instance("fan") {
                if let Some(DeviceParameters::Integer {
                    range: IntegerRange { min, max, .. },
                    unit,
                }) = &cap.parameters
                {
                    if unit.as_deref() == Some("unit.percent") {
                        speed_range_min = Some(*min as u8);
                        speed_range_max = Some(*max as u8);
                    }
                }
            }
        }

        // For BLE-only fans without Platform API (e.g. H7105), default to 12 speeds.
        // homebridge-govee confirms H7105 supports speeds 1–12 via ptReal 33 05 01 <speed>.
        if speed_range_min.is_none() && speed_range_max.is_none() && device.http_device_info.is_none() {
            speed_range_min = Some(1);
            speed_range_max = Some(12);
        }

        let work_mode = ParsedWorkMode::with_device(device).ok();
        let preset_modes = work_mode
            .as_ref()
            .map(|wm| wm.get_mode_names())
            .unwrap_or_default();

        Ok(Self {
            fan: FanConfig {
                base: EntityConfig {
                    availability_topic: availability_topic(),
                    name: if matches!(device.device_type(), DeviceType::Fan) {
                        None
                    } else {
                        Some("Fan".to_string())
                    },
                    device_class: Some("fan"),
                    origin: Origin::default(),
                    device: Device::for_device(device),
                    unique_id,
                    entity_category: None,
                    icon: None,
                },
                command_topic,
                speed_range_min,
                speed_range_max,
                percentage_command_topic,
                percentage_state_topic,
                mode_command_topic,
                mode_state_topic,
                preset_modes,
                state_topic,
                optimistic,
            },
            device_id: device.id.to_string(),
            state: state.clone(),
        })
    }
}

#[async_trait]
impl EntityInstance for Fan {
    async fn publish_config(&self, state: &StateHandle, client: &HassClient) -> anyhow::Result<()> {
        publish_entity_config("fan", state, client, &self.fan.base, &self.fan).await
    }

    async fn notify_state(&self, client: &HassClient) -> anyhow::Result<()> {
        let device = self
            .state
            .device_by_id(&self.device_id)
            .await
            .expect("device to exist");

        // Broadcast power state
        let is_on = device.device_state().map(|s| s.on).unwrap_or(false);
        client
            .publish(
                &self.fan.state_topic,
                if is_on { "ON" } else { "OFF" },
            )
            .await?;

        // Broadcast speed
        if let Some(speed) = device.target_fan_speed {
            client
                .publish(&self.fan.percentage_state_topic, speed.to_string())
                .await?;
        } else {
            let guessed = self.fan.speed_range_min.unwrap_or(0);
            self.state
                .device_mut(&device.sku, &device.id)
                .await
                .set_fan_speed(guessed);
            client
                .publish(&self.fan.percentage_state_topic, guessed.to_string())
                .await?;
        }

        // Broadcast current preset mode from device capability state
        if let Ok(work_modes) = ParsedWorkMode::with_device(&device) {
            if let Some(cap) = device.get_state_capability_by_instance("workMode") {
                if let Some(mode_num) = cap.state.pointer("/value/workMode") {
                    if let Some(mode) = work_modes.mode_for_value(mode_num) {
                        client
                            .publish(&self.fan.mode_state_topic, mode.name.to_string())
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }
}

pub async fn mqtt_fan_set_work_mode(
    Payload(mode): Payload<String>,
    Params(IdParameter { id }): Params<IdParameter>,
    State(state): State<StateHandle>,
) -> anyhow::Result<()> {
    log::info!("mqtt_fan_set_work_mode: {id}: {mode}");
    let device = state.resolve_device_for_control(&id).await?;

    let work_modes = ParsedWorkMode::with_device(&device)?;
    let work_mode = work_modes
        .mode_by_name(&mode)
        .ok_or_else(|| anyhow!("mode {mode} not found"))?;
    let mode_num = work_mode
        .value
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("expected workMode to be a number"))?;
    let value = work_mode.default_value();

    state
        .humidifier_set_parameter(&device, mode_num, value)
        .await?;

    Ok(())
}

pub async fn mqtt_fan_set_speed(
    Payload(percent): Payload<i64>,
    Params(IdParameter { id }): Params<IdParameter>,
    State(state): State<StateHandle>,
) -> anyhow::Result<()> {
    log::info!("mqtt_fan_set_speed: {id}: {percent}");
    let device = state.resolve_device_for_control(&id).await?;

    state.fan_set_speed(&device, percent).await?;

    state
        .device_mut(&device.sku, &device.id)
        .await
        .set_fan_speed(percent as u8);

    Ok(())
}

pub async fn mqtt_fan_set_oscillation(
    Payload(oscillate): Payload<String>,
    Params(IdParameter { id }): Params<IdParameter>,
    State(state): State<StateHandle>,
) -> anyhow::Result<()> {
    // Home Assistant sends "oscillate" or "fixed"
    let on = oscillate.eq_ignore_ascii_case("oscillate");
    log::info!("mqtt_fan_set_oscillation: {id}: {on}");
    let device = state.resolve_device_for_control(&id).await?;

    state.fan_set_oscillate(&device, on).await?;

    Ok(())
}
