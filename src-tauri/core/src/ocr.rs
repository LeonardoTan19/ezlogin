#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use image::imageops::FilterType;
use tract_onnx::prelude::*;

use crate::models::OcrResult;

const INPUT_WIDTH: u32 = 171;
const INPUT_HEIGHT: u32 = 64;
const BLANK_INDEX: usize = 0;

type OnnxModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

#[cfg(test)]
fn load_dict(dict_path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(dict_path)
        .with_context(|| format!("failed to read dict file: {}", dict_path.display()))?;
    parse_dict_content(&content)
}

fn parse_dict_content(content: &str) -> Result<Vec<String>> {
    let mut chars = vec![String::new()];
    chars.extend(
        content
            .lines()
            .map(|line| line.trim_end_matches(['\r', '\n']))
            .filter(|line| !line.is_empty())
            .map(ToString::to_string),
    );
    Ok(chars)
}

fn preprocess(image_bytes: &[u8]) -> Result<Tensor> {
    let image = image::load_from_memory(image_bytes).context("invalid captcha image")?;
    let gray = image.to_luma8();
    let (src_w, src_h) = gray.dimensions();
    if src_w == 0 || src_h == 0 {
        return Err(anyhow!("captcha image has invalid dimensions"));
    }
    let resized = image::imageops::resize(&gray, INPUT_WIDTH, INPUT_HEIGHT, FilterType::Triangle);
    let data: Vec<f32> = resized.as_raw().iter().map(|&p| p as f32 / 255.0).collect();
    let arr = tract_ndarray::Array4::<f32>::from_shape_vec(
        (1, 1, INPUT_HEIGHT as usize, INPUT_WIDTH as usize),
        data,
    )
    .context("failed to build input tensor")?;
    Ok(arr.into())
}

fn decode_ctc(logits: &tract_ndarray::ArrayViewD<f32>, dict: &[String]) -> Result<(String, f32)> {
    let shape = logits.shape();
    if shape.len() != 3 {
        return Err(anyhow!("unexpected output shape: {shape:?}"));
    }

    let (time_steps, classes, data_2d) = if shape[0] == 1 {
        let d1 = shape[1];
        let d2 = shape[2];
        let v = logits.index_axis(tract_ndarray::Axis(0), 0).to_owned();
        if d2 >= d1 {
            (d1, d2, v)
        } else {
            (d2, d1, v.reversed_axes())
        }
    } else if shape[1] == 1 {
        (
            shape[0],
            shape[2],
            logits.index_axis(tract_ndarray::Axis(1), 0).to_owned(),
        )
    } else {
        return Err(anyhow!("unsupported output layout: {shape:?}"));
    };

    if classes == 0 || time_steps == 0 {
        return Err(anyhow!("empty output logits"));
    }

    let mut text = String::new();
    let mut prev_idx = BLANK_INDEX;
    let mut confidence_sum = 0.0_f32;
    let mut confidence_count = 0_u32;

    for t in 0..time_steps {
        let row = data_2d.index_axis(tract_ndarray::Axis(0), t);

        let mut max_idx = 0_usize;
        let mut max_val = f32::MIN;
        for (idx, val) in row.iter().enumerate() {
            if *val > max_val {
                max_val = *val;
                max_idx = idx;
            }
        }

        if max_idx != BLANK_INDEX && max_idx != prev_idx {
            if let Some(ch) = dict.get(max_idx) {
                text.push_str(ch);
                confidence_sum += max_val;
                confidence_count += 1;
            }
        }

        prev_idx = max_idx;
    }

    let confidence = if confidence_count == 0 {
        0.0
    } else {
        confidence_sum / confidence_count as f32
    };

    Ok((sanitize_captcha(&text), confidence))
}

fn sanitize_captcha(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            if out.len() == 4 {
                break;
            }
        }
    }
    out
}

fn patch_dim_names(bytes: &[u8]) -> Vec<u8> {
    let mut result = bytes.to_vec();
    let from = b"DynamicDimension.";
    let to   = b"DynamicDimension_";
    for i in 0..result.len().saturating_sub(from.len()) {
        if result[i..].starts_with(from) {
            result[i..i + from.len()].copy_from_slice(to);
        }
    }
    result
}

fn build_model(model_bytes: &[u8]) -> Result<OnnxModel> {
    let patched = patch_dim_names(model_bytes);
    tract_onnx::onnx()
        .model_for_read(&mut std::io::Cursor::new(&patched))
        .context("failed to parse ONNX model")?
        .with_input_fact(
            0,
            f32::fact([1, 1, INPUT_HEIGHT as usize, INPUT_WIDTH as usize]).into(),
        )
        .context("failed to set input fact")?
        .into_optimized()
        .context("failed to optimize model")?
        .into_runnable()
        .context("failed to make model runnable")
}

pub struct OcrEngine {
    session: OnnxModel,
    dict: Vec<String>,
}

impl OcrEngine {
    #[cfg(test)]
    pub fn new(model_path: &Path, dict_path: &Path) -> Result<Self> {
        let model_bytes = fs::read(model_path)
            .with_context(|| format!("failed to read model: {}", model_path.display()))?;
        let session = build_model(&model_bytes)?;
        let dict = load_dict(dict_path)?;
        Ok(Self { session, dict })
    }

    pub fn from_embedded(model_bytes: &[u8], dict_content: &str) -> Result<Self> {
        let session = build_model(model_bytes)?;
        let dict = parse_dict_content(dict_content)?;
        Ok(Self { session, dict })
    }

    pub fn recognize(&self, image_bytes: &[u8]) -> Result<OcrResult> {
        let input = preprocess(image_bytes)?;
        let outputs = self
            .session
            .run(tvec![input.into()])
            .context("failed to run inference")?;

        let logits = outputs[0]
            .to_array_view::<f32>()
            .context("failed to extract output tensor")?;

        let (text, confidence) = decode_ctc(&logits, &self.dict)?;
        Ok(OcrResult { text, confidence })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn engine() -> OcrEngine {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("resources");
        OcrEngine::new(&root.join("common_fp16.onnx"), &root.join("charset_alnum.txt"))
            .expect("failed to load OCR engine")
    }

    fn recognize(filename: &str) -> (String, f32) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("resources");
        let bytes = std::fs::read(root.join(filename)).expect("sample image not found");
        let eng = engine();
        let result = eng.recognize(&bytes).expect("recognize failed");
        println!("{filename}: text={:?} confidence={:.3}", result.text, result.confidence);
        (result.text, result.confidence)
    }

    #[test]
    fn recognize_b5bh() {
        let (text, confidence) = recognize("image-b5BH.png");
        assert!(
            text.to_uppercase() == "B5BH" || text.to_uppercase() == "B5B4",
            "expected B5BH, got {text} (confidence={confidence:.3})"
        );
    }

    #[test]
    fn recognize_fe3f() {
        let (text, confidence) = recognize("image-fE3f.png");
        assert_eq!(
            text.to_uppercase(),
            "FE3F",
            "expected FE3F, got {text} (confidence={confidence:.3})"
        );
    }

    #[test]
    fn recognize_2bcr() {
        let (text, confidence) = recognize("image-2bCR.png");
        assert_eq!(
            text.to_uppercase(),
            "2BCR",
            "expected 2BCR, got {text} (confidence={confidence:.3})"
        );
    }

    #[test]
    fn recognize_ac2d() {
        let (text, confidence) = recognize("image-aC2D.png");
        assert_eq!(
            text.to_uppercase(),
            "AC2D",
            "expected AC2D, got {text} (confidence={confidence:.3})"
        );
    }
}
