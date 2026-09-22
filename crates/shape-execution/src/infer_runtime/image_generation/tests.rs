use std::{collections::BTreeMap, io::Cursor};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use infer_runtime_client::ResponsesResult;
use serde_json::json;
use shape_domain::{
    AiImageGenerateParameters, AiImageOutputCanvas, ArtifactContentContract, TransformationId,
};

use super::{
    IMAGE_GENERATE_CAPABILITY, IMAGE_MEDIA_TYPE, InferRuntimeImageGenerationExecutor,
    image_deployment, image_effort, image_request, image_request_for_deployment,
};
use crate::infer_runtime::{job_provenance::tests::image_job, test_support::FakeSdk};
use crate::{CapabilityId, ExecutionRequest, Executor as _};

fn png(width: u32, height: u32) -> Vec<u8> {
    let pixels = vec![127_u8; width as usize * height as usize * 4];
    let mut bytes = Cursor::new(Vec::new());
    PngEncoder::new(&mut bytes)
        .write_image(&pixels, width, height, ColorType::Rgba8.into())
        .unwrap();
    bytes.into_inner()
}

fn parameters(width: u32, height: u32, count: u8) -> AiImageGenerateParameters {
    AiImageGenerateParameters::new(
        "A pale blue circle on white, without text",
        AiImageOutputCanvas::new(width, height).unwrap(),
        count,
        Vec::new(),
    )
    .unwrap()
}

fn request(parameters: &AiImageGenerateParameters) -> ExecutionRequest {
    ExecutionRequest::new(
        TransformationId::new(),
        CapabilityId::new(IMAGE_GENERATE_CAPABILITY).unwrap(),
        Vec::new(),
        serde_json::to_vec(parameters).unwrap(),
        IMAGE_MEDIA_TYPE,
    )
    .unwrap()
}

fn response(encoded: &str) -> ResponsesResult {
    ResponsesResult {
        id: "resp_shape_job".into(),
        object: "response".into(),
        created_at: 1,
        model: "image.generate".into(),
        status: "completed".into(),
        output: vec![json!({
            "id":"image_1",
            "type":"image_generation_call",
            "status":"completed",
            "result":encoded,
            "future_item_field":true
        })],
        extra: BTreeMap::default(),
    }
}

#[test]
fn source_less_image_generation_uses_stable_responses_capability_shape() {
    let request = image_request(&parameters(1536, 1024, 1));
    assert_eq!(request.model, "image.generate");
    assert_eq!(
        request.metadata["infer.deployment_ids"],
        "codex_gpt_5_6_luna"
    );
    assert!(
        request
            .instructions
            .as_ref()
            .unwrap()
            .as_str()
            .unwrap()
            .contains("1536 by 1024")
    );
    assert_eq!(request.tools, vec![json!({"type":"image_generation"})]);
    assert_eq!(request.metadata["infer.placement"], "cloud_only");
    assert_eq!(
        request.metadata["infer.provider_access_class"],
        "subscription"
    );
    assert_eq!(request.metadata["infer.fallback"], "none");
    assert_eq!(request.metadata["infer.max_cost_usd"], "0");
}

#[test]
fn cloud_image_choices_keep_the_selected_deployment() {
    for (key, deployment) in [
        ("gpt_6_luna", "codex_gpt_6_luna"),
        ("gpt_6_sol", "codex_gpt_6_sol"),
    ] {
        assert_eq!(image_deployment(key), Some(deployment));
        let request =
            image_request_for_deployment(&parameters(1024, 1024, 1), deployment, Some("high"));
        assert_eq!(request.metadata["infer.deployment_ids"], deployment);
        assert_eq!(request.metadata["infer.fallback"], "none");
        assert_eq!(request.reasoning, Some(json!({"effort": "high"})));
    }
    assert_eq!(image_deployment("arbitrary_deployment"), None);
    assert_eq!(image_effort("gpt_6_luna", "ultra"), Err(()));
    assert_eq!(image_effort("gpt_6_sol", "ultra"), Ok(Some("ultra")));
    assert_eq!(image_effort("gpt_5_6_luna", ""), Ok(None));
}

#[test]
fn sdk_image_response_is_revalidated_before_transient_output() {
    let encoded = STANDARD.encode(png(4, 3));
    let fake = FakeSdk::new().response(response(&encoded)).job(image_job());
    let executor = InferRuntimeImageGenerationExecutor::with_sdk(Box::new(fake));
    let output = executor
        .execute(&request(&parameters(4, 3, 1)))
        .expect("valid generated PNG becomes an execution output");
    assert_eq!(output.media_type, "image/png");
    assert_eq!(output.executor_job_id.as_deref(), Some("resp_shape_job"));
    let Some(ArtifactContentContract::ImageRaster(contract)) = output.content_contract else {
        panic!("image contract missing");
    };
    assert_eq!((contract.width, contract.height), (4, 3));
}

#[test]
fn image_edit_and_multi_candidate_requests_remain_dependency_gated() {
    let executor = InferRuntimeImageGenerationExecutor::with_sdk(Box::new(FakeSdk::new()));
    assert!(!executor.supports(&CapabilityId::new("image.edit").unwrap()));
    let error = executor
        .execute(&request(&parameters(4, 3, 2)))
        .expect_err("multi-candidate transport remains unsupported");
    assert_eq!(error.code, "unsupported_image_candidate_count");
}

#[test]
fn generated_native_geometry_is_preserved_without_resampling() {
    let encoded = STANDARD.encode(png(5, 3));
    let fake = FakeSdk::new().response(response(&encoded)).job(image_job());
    let output = InferRuntimeImageGenerationExecutor::with_sdk(Box::new(fake))
        .execute(&request(&parameters(4, 3, 1)))
        .expect("valid native geometry remains a reviewable image");
    let Some(ArtifactContentContract::ImageRaster(contract)) = output.content_contract else {
        panic!("image contract missing");
    };
    assert_eq!((contract.width, contract.height), (5, 3));
    assert_eq!(
        image::load_from_memory(&output.bytes).unwrap().to_rgba8(),
        image::load_from_memory(&png(5, 3)).unwrap().to_rgba8()
    );
}

#[test]
fn image_generation_rejects_a_successful_job_routed_to_an_unrequested_model() {
    let encoded = STANDARD.encode(png(4, 3));
    let mut job = image_job();
    job.deployment = "codex_gpt_5_6_sol".into();
    job.attempts[0].deployment = job.deployment.clone();
    let fake = FakeSdk::new().response(response(&encoded)).job(job);
    let error = InferRuntimeImageGenerationExecutor::with_sdk(Box::new(fake))
        .execute(&request(&parameters(4, 3, 1)))
        .expect_err("a more expensive unrequested model cannot substitute for Luna");
    assert_eq!(error.code, "infer_policy_violation");
}
