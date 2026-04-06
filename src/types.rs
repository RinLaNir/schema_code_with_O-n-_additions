use crate::code::AdditiveCode;
use ldpc_toolbox::codes::ccsds::{AR4JAInfoSize, AR4JARate};
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use ldpc_toolbox::gf2::GF2;
use ndarray::Array1;
use rand::RngExt;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt::{Debug, Formatter};
use std::time::Duration;

/// Serde module for `Duration` fields serialized as milliseconds.
pub mod duration_as_ms {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(duration.as_secs_f64() * 1000.0)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let ms = f64::deserialize(deserializer)?;
        Ok(Duration::from_secs_f64(ms / 1000.0))
    }
}

/// Serialize `Duration` as milliseconds for serialize-only fields.
pub fn serialize_duration_as_ms<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    duration_as_ms::serialize(duration, serializer)
}

fn serialize_option_debug<T: Debug, S: Serializer>(
    val: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match val {
        Some(v) => serializer.serialize_str(&format!("{:?}", v)),
        None => serializer.serialize_none(),
    }
}

/// Packed little-endian representation of an element of `F_{2^ell}`.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct F2PowElement {
    pub bytes: Vec<u8>,
    pub bit_len: usize,
}

impl Debug for F2PowElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "F2PowElement(0x{}, {} bits)",
            self.to_hex(),
            self.bit_len
        )
    }
}

impl F2PowElement {
    pub fn zero(bit_len: usize) -> Self {
        Self {
            bytes: vec![0; bit_len.div_ceil(8)],
            bit_len,
        }
    }

    fn from_bytes_le(mut bytes: Vec<u8>, bit_len: usize) -> Self {
        bytes.resize(bit_len.div_ceil(8), 0);
        let mut element = Self { bytes, bit_len };
        element.clear_unused_bits();
        element
    }

    #[cfg(test)]
    pub fn from_u128(value: u128, bit_len: usize) -> Self {
        Self::from_bytes_le(value.to_le_bytes().to_vec(), bit_len)
    }

    pub fn random<R: rand::Rng + ?Sized>(bit_len: usize, rng: &mut R) -> Self {
        let mut bytes = vec![0; bit_len.div_ceil(8)];
        rng.fill(&mut bytes[..]);
        let mut element = Self { bytes, bit_len };
        element.clear_unused_bits();
        element
    }

    pub fn bit(&self, index: usize) -> bool {
        assert!(index < self.bit_len, "bit index {} out of range", index);
        let byte = self.bytes[index / 8];
        ((byte >> (index % 8)) & 1) == 1
    }

    pub fn set_bit(&mut self, index: usize, value: bool) {
        assert!(index < self.bit_len, "bit index {} out of range", index);
        let mask = 1u8 << (index % 8);
        if value {
            self.bytes[index / 8] |= mask;
        } else {
            self.bytes[index / 8] &= !mask;
        }
    }

    pub fn xor_assign(&mut self, other: &Self) {
        assert_eq!(
            self.bit_len, other.bit_len,
            "bit lengths must match for XOR"
        );
        for (lhs, rhs) in self.bytes.iter_mut().zip(&other.bytes) {
            *lhs ^= *rhs;
        }
        self.clear_unused_bits();
    }

    pub fn from_hex(hex: &str, bit_len: usize) -> Result<Self, String> {
        let trimmed = hex.trim();
        let stripped = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
            .unwrap_or(trimmed)
            .replace('_', "");
        let normalized = if stripped.is_empty() {
            String::from("0")
        } else {
            stripped
        };

        if !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Invalid hex string: {}", hex));
        }

        let padded = if normalized.len().is_multiple_of(2) {
            normalized
        } else {
            format!("0{}", normalized)
        };

        let byte_len = bit_len.div_ceil(8);
        let parsed_len = padded.len() / 2;
        if parsed_len > byte_len {
            return Err(format!("Hex secret exceeds {} bits", bit_len));
        }

        let mut be_bytes = vec![0u8; byte_len];
        let offset = byte_len - parsed_len;
        for i in 0..parsed_len {
            let start = i * 2;
            let end = start + 2;
            be_bytes[offset + i] = u8::from_str_radix(&padded[start..end], 16)
                .map_err(|err| format!("Invalid hex byte: {}", err))?;
        }

        if let Some(mask) = Self::last_byte_mask(bit_len) {
            let last = be_bytes.first().copied().unwrap_or(0);
            if (last & !mask) != 0 {
                return Err(format!("Hex secret exceeds {} bits", bit_len));
            }
        }

        be_bytes.reverse();
        let mut element = Self::from_bytes_le(be_bytes, bit_len);
        element.clear_unused_bits();
        Ok(element)
    }

    pub fn to_hex(&self) -> String {
        let mut be_bytes = self.bytes.clone();
        be_bytes.reverse();
        if be_bytes.is_empty() {
            return String::from("00");
        }
        be_bytes
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect()
    }

    fn last_byte_mask(bit_len: usize) -> Option<u8> {
        let used_bits = bit_len % 8;
        if used_bits == 0 || bit_len == 0 {
            None
        } else {
            Some((1u8 << used_bits) - 1)
        }
    }

    fn clear_unused_bits(&mut self) {
        if let Some(mask) = Self::last_byte_mask(self.bit_len) {
            if let Some(last) = self.bytes.last_mut() {
                *last &= mask;
            }
        }
    }
}

const DECODER_VARIANTS: &[(DecoderImplementation, &str)] = &[
    (DecoderImplementation::Phif64, "Phif64"),
    (DecoderImplementation::Phif32, "Phif32"),
    (DecoderImplementation::Tanhf64, "Tanhf64"),
    (DecoderImplementation::Tanhf32, "Tanhf32"),
    (DecoderImplementation::Minstarapproxf64, "Minstarapproxf64"),
    (DecoderImplementation::Minstarapproxf32, "Minstarapproxf32"),
    (DecoderImplementation::Minstarapproxi8, "Minstarapproxi8"),
    (
        DecoderImplementation::Minstarapproxi8Jones,
        "Minstarapproxi8Jones",
    ),
    (
        DecoderImplementation::Minstarapproxi8PartialHardLimit,
        "Minstarapproxi8PartialHardLimit",
    ),
    (
        DecoderImplementation::Minstarapproxi8JonesPartialHardLimit,
        "Minstarapproxi8JonesPartialHardLimit",
    ),
    (
        DecoderImplementation::Minstarapproxi8Deg1Clip,
        "Minstarapproxi8Deg1Clip",
    ),
    (
        DecoderImplementation::Minstarapproxi8JonesDeg1Clip,
        "Minstarapproxi8JonesDeg1Clip",
    ),
    (
        DecoderImplementation::Minstarapproxi8PartialHardLimitDeg1Clip,
        "Minstarapproxi8PartialHardLimitDeg1Clip",
    ),
    (
        DecoderImplementation::Minstarapproxi8JonesPartialHardLimitDeg1Clip,
        "Minstarapproxi8JonesPartialHardLimitDeg1Clip",
    ),
    (DecoderImplementation::Aminstarf64, "Aminstarf64"),
    (DecoderImplementation::Aminstarf32, "Aminstarf32"),
    (DecoderImplementation::Aminstari8, "Aminstari8"),
    (DecoderImplementation::Aminstari8Jones, "Aminstari8Jones"),
    (
        DecoderImplementation::Aminstari8PartialHardLimit,
        "Aminstari8PartialHardLimit",
    ),
    (
        DecoderImplementation::Aminstari8JonesPartialHardLimit,
        "Aminstari8JonesPartialHardLimit",
    ),
    (
        DecoderImplementation::Aminstari8Deg1Clip,
        "Aminstari8Deg1Clip",
    ),
    (
        DecoderImplementation::Aminstari8JonesDeg1Clip,
        "Aminstari8JonesDeg1Clip",
    ),
    (
        DecoderImplementation::Aminstari8PartialHardLimitDeg1Clip,
        "Aminstari8PartialHardLimitDeg1Clip",
    ),
    (
        DecoderImplementation::Aminstari8JonesPartialHardLimitDeg1Clip,
        "Aminstari8JonesPartialHardLimitDeg1Clip",
    ),
    (DecoderImplementation::HLPhif64, "HLPhif64"),
    (DecoderImplementation::HLPhif32, "HLPhif32"),
    (DecoderImplementation::HLTanhf64, "HLTanhf64"),
    (DecoderImplementation::HLTanhf32, "HLTanhf32"),
    (
        DecoderImplementation::HLMinstarapproxf64,
        "HLMinstarapproxf64",
    ),
    (
        DecoderImplementation::HLMinstarapproxf32,
        "HLMinstarapproxf32",
    ),
    (
        DecoderImplementation::HLMinstarapproxi8,
        "HLMinstarapproxi8",
    ),
    (
        DecoderImplementation::HLMinstarapproxi8PartialHardLimit,
        "HLMinstarapproxi8PartialHardLimit",
    ),
    (DecoderImplementation::HLAminstarf64, "HLAminstarf64"),
    (DecoderImplementation::HLAminstarf32, "HLAminstarf32"),
    (DecoderImplementation::HLAminstari8, "HLAminstari8"),
    (
        DecoderImplementation::HLAminstari8PartialHardLimit,
        "HLAminstari8PartialHardLimit",
    ),
];

pub fn decoder_variants() -> &'static [(DecoderImplementation, &'static str)] {
    DECODER_VARIANTS
}

pub fn all_decoder_types() -> Vec<DecoderImplementation> {
    DECODER_VARIANTS
        .iter()
        .map(|(decoder, _)| *decoder)
        .collect()
}

pub fn parse_decoder_type(s: &str) -> Result<DecoderImplementation, String> {
    DECODER_VARIANTS
        .iter()
        .find(|(_, name)| *name == s)
        .map(|(decoder, _)| *decoder)
        .ok_or_else(|| format!("Unknown decoder type: {}", s))
}

pub fn parse_ldpc_rate(s: &str) -> Result<AR4JARate, String> {
    match s {
        "R1_2" | "1_2" => Ok(AR4JARate::R1_2),
        "R2_3" | "2_3" => Ok(AR4JARate::R2_3),
        "R4_5" | "4_5" => Ok(AR4JARate::R4_5),
        _ => Err(format!("Unknown LDPC rate: {}", s)),
    }
}

pub fn parse_ldpc_info_size(s: &str) -> Result<AR4JAInfoSize, String> {
    match s {
        "K1024" => Ok(AR4JAInfoSize::K1024),
        "K4096" => Ok(AR4JAInfoSize::K4096),
        "K16384" => Ok(AR4JAInfoSize::K16384),
        _ => Err(format!("Unknown LDPC info size: {}", s)),
    }
}

pub fn info_bits(info_size: AR4JAInfoSize) -> usize {
    match info_size {
        AR4JAInfoSize::K1024 => 1024,
        AR4JAInfoSize::K4096 => 4096,
        AR4JAInfoSize::K16384 => 16384,
    }
}

pub fn codeword_bits(rate: AR4JARate, info_size: AR4JAInfoSize) -> usize {
    let k = info_bits(info_size);
    match rate {
        AR4JARate::R1_2 => k * 2,
        AR4JARate::R2_3 => (k * 3) / 2,
        AR4JARate::R4_5 => (k * 5) / 4,
    }
}

#[derive(Clone, Serialize)]
pub struct CodeInitParams {
    #[serde(serialize_with = "serialize_option_debug")]
    pub decoder_type: Option<DecoderImplementation>,
    #[serde(serialize_with = "serialize_option_debug")]
    pub ldpc_rate: Option<AR4JARate>,
    #[serde(serialize_with = "serialize_option_debug")]
    pub ldpc_info_size: Option<AR4JAInfoSize>,
    pub max_iterations: Option<usize>,
    pub llr_value: Option<f64>,
    pub secret_bits: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseMetrics {
    #[allow(dead_code)]
    #[serde(default)]
    pub name: String,
    #[serde(with = "duration_as_ms", default)]
    pub duration: Duration,
    #[serde(default)]
    pub percentage: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DealMetrics {
    pub rand_vec_generation: PhaseMetrics,
    pub mask_xor: PhaseMetrics,
    pub matrix_creation: PhaseMetrics,
    pub encoding: PhaseMetrics,
    pub share_creation: PhaseMetrics,
    #[allow(dead_code)]
    #[serde(with = "duration_as_ms", default)]
    pub total_time: Duration,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecodingStats {
    pub total_rows: usize,
    pub successful_rows: usize,
    pub failed_rows: usize,
    pub total_iterations: usize,
    pub avg_iterations: f64,
    pub max_iterations_hit: usize,
}

impl DecodingStats {
    pub fn new(
        total_rows: usize,
        successful_rows: usize,
        failed_rows: usize,
        total_iterations: usize,
        max_iterations_hit: usize,
    ) -> Self {
        let avg_iterations = if successful_rows > 0 {
            total_iterations as f64 / successful_rows as f64
        } else {
            0.0
        };

        Self {
            total_rows,
            successful_rows,
            failed_rows,
            total_iterations,
            avg_iterations,
            max_iterations_hit,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_rows > 0 {
            self.successful_rows as f64 / self.total_rows as f64
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReconstructMetrics {
    pub matrix_setup: PhaseMetrics,
    pub row_decoding: PhaseMetrics,
    pub column_reconstruction: PhaseMetrics,
    pub final_computation: PhaseMetrics,
    #[allow(dead_code)]
    #[serde(with = "duration_as_ms", default)]
    pub total_time: Duration,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decoding_stats: Option<DecodingStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParallelMetrics {
    pub thread_count: usize,
    pub speedup: Option<f64>,
    pub efficiency: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    pub shares_per_second: f64,
    pub bits_per_second: f64,
}

impl PhaseMetrics {
    pub fn new(name: &str, duration: Duration, total_time: Duration) -> Self {
        let total_nanos = total_time.as_nanos();
        let percentage = if total_nanos > 0 {
            duration.as_nanos() as f64 / total_nanos as f64 * 100.0
        } else {
            0.0
        };

        Self {
            name: name.to_string(),
            duration,
            percentage,
        }
    }
}

pub struct CodeParams<C: AdditiveCode> {
    pub output_length: u32,
    pub input_length: u32,
    pub code_impl: C,
}

pub struct SecretParams<C: AdditiveCode> {
    pub code: CodeParams<C>,
    pub ell: usize,
    pub a_bits: Vec<bool>,
}

#[derive(Clone)]
pub struct Shares {
    pub shares: Vec<Share>,
    pub z0: F2PowElement,
    pub metrics: Option<DealMetrics>,
}

#[derive(Clone)]
pub struct Share {
    pub y: Array1<GF2>,
    pub i: u32,
}

#[cfg(test)]
mod tests {
    use super::F2PowElement;

    #[test]
    fn test_from_hex_pads_to_bit_length() {
        let element = F2PowElement::from_hex("0x2a", 16).unwrap();
        assert_eq!(element.to_hex(), "002a");
        assert!(element.bit(1));
        assert!(element.bit(3));
        assert!(element.bit(5));
    }

    #[test]
    fn test_from_hex_rejects_out_of_range_bits() {
        let err = F2PowElement::from_hex("0200", 9).unwrap_err();
        assert!(err.contains("exceeds"));
    }

    #[test]
    fn test_xor_assign() {
        let mut lhs = F2PowElement::from_hex("0f0f", 16).unwrap();
        let rhs = F2PowElement::from_hex("00ff", 16).unwrap();
        lhs.xor_assign(&rhs);
        assert_eq!(lhs.to_hex(), "0ff0");
    }

    #[test]
    fn test_set_and_get_bits() {
        let mut element = F2PowElement::zero(10);
        element.set_bit(0, true);
        element.set_bit(9, true);
        assert!(element.bit(0));
        assert!(element.bit(9));
        assert_eq!(element.to_hex(), "0201");
    }
}
