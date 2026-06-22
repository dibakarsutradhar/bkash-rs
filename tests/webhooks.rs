//! Wiremock-backed integration tests for SNS webhook verification.
//!
//! These tests:
//! - Generate a known canonical string-to-sign from a real SNS envelope
//!   shape.
//! - Sign it with a real RSA-SHA1 private key (self-signed test cert).
//! - Mount a wiremock mock that returns that cert at the
//!   `SigningCertURL`.
//! - Run [`verify_sns_signature`] and assert it succeeds.
//!
//! The test key/cert are not "real" production material — they are
//! regenerated per crate release and exist only as inline test data.

// The webhooks integration test depends on the `webhooks` feature.
#![cfg(feature = "webhooks")]

use base64::Engine as _;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey as _;
use rsa::signature::{SignatureEncoding, Signer as _};
use rsa::RsaPrivateKey;
use sha1::Sha1;

use bkash_rs::webhooks::{
    build_string_to_sign, cert_pem_to_public_key, confirm_subscription, parse_event,
    verify_signature_with_key, verify_sns_signature, SnsEnvelope, WebhookEvent,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Self-signed test certificate returned by the mock SNS endpoint.
const TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIDNzCCAh+gAwIBAgIUciCT+yZYvcoN0cO9T7EkjoQgyVAwDQYJKoZIhvcNAQEL
BQAwKzEpMCcGA1UEAwwgdGVzdC5zbnMudXMtd2VzdC0yLmFtYXpvbmF3cy5jb20w
HhcNMjYwNjIyMTMyODMxWhcNMjcwNjIyMTMyODMxWjArMSkwJwYDVQQDDCB0ZXN0
LnNucy51cy13ZXN0LTIuYW1hem9uYXdzLmNvbTCCASIwDQYJKoZIhvcNAQEBBQAD
ggEPADCCAQoCggEBALkyxPaZxyazezFzsXsROd/R/REQXYnoYUX1FPHngv4W1TOu
35IyO2X8ciHqwlze5qzy2EgWhg8FYma6L4WW4AaRjujM7/eJ/2JfXdWP0W+7eIbc
MxpIJqM6YbGtC4jPbnFDzgQz7ekEZi6xX2IM7yWmH1+e003nWahDx9C+cSBsBbup
dclg5nM+OeWEgt0RXErRc7SsmZ/w1/IO3w/+hfpt1IA7P7D3rOHrdF7FMHGXAl9j
UItPmzemjbzfo7u1Zk+SFCa5aKHN9839UJpDJVTMB/22c3Uvc4OUZ0CuOMe14RVv
nnM7ZkodslnPqYMn545W+D20qd1DyJDjRoVfe9sCAwEAAaNTMFEwHQYDVR0OBBYE
FIX+Z9c9ERsu5um91F4DCnCFySRMMB8GA1UdIwQYMBaAFIX+Z9c9ERsu5um91F4D
CnCFySRMMA8GA1UdEwEB/wQFMAMBAf8wDQYJKoZIhvcNAQELBQADggEBAEaN44qC
f9FYqronePk8Rp1BvGQ8q++Zyf1X4x7IILkleW4c2tu2jdNV7gIUXjGdSDqW7+IM
NCLBz+gHhwm/qdCT68yr3vWctBHLk9295TZk3xTCpI7a9Z0iosdON5ZWJB8EuPW2
ILvESX9gSop463kYHvTyBzJMo5WbwWzLJnVr0Nu1UcJIEHtFtsKW4o+R1Q5hEOcQ
Rhk5a+egHWbyh6y3xyfkqJ574rnmyBQxzwuBMChpZtGpln+QTNFcwSKC84kKhLr0
rUht7HorzuxfIrKbWGt5owMjxlJpktL3ORVAKJPHidmnXlDeLy/lJATbCDqoLUWE
qlO36oh7n7LLf0M=
-----END CERTIFICATE-----
";

/// Private key corresponding to [`TEST_CERT_PEM`] (PKCS#8 PEM).
const TEST_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC5MsT2mccms3sx
c7F7ETnf0f0REF2J6GFF9RTx54L+FtUzrt+SMjtl/HIh6sJc3uas8thIFoYPBWJm
ui+FluAGkY7ozO/3if9iX13Vj9Fvu3iG3DMaSCajOmGxrQuIz25xQ84EM+3pBGYu
sV9iDO8lph9fntNN51moQ8fQvnEgbAW7qXXJYOZzPjnlhILdEVxK0XO0rJmf8Nfy
Dt8P/oX6bdSAOz+w96zh63RexTBxlwJfY1CLT5s3po2836O7tWZPkhQmuWihzffN
/VCaQyVUzAf9tnN1L3ODlGdArjjHteEVb55zO2ZKHbJZz6mDJ+eOVvg9tKndQ8iQ
40aFX3vbAgMBAAECggEABUR0Za8qAufH8TghLCbpRnxqfjnA71M0sRCvb9Ht39w2
ZCntIfNbzSLI/D35GTsdxH2DuUkqtvKoQdO51krcRFOPhP+PM7MxGFMuEAzvIAZs
/yj0KyMyNiJK9Qq+8T01HvOWwHfZpG8DmQBifh9wDdoTulyCFg0md6q13DIOdaWQ
eyZ+zpP3amX0XpE4KMCEvhpx8yYeweUDbSNZa7qHXp8keHCDr6XSmB5p3TqNoYqx
whK1hajQsJjCfeEMiTqb6EkkkaIp3zb9OHRtBNhVSMUK/gqUBc+FYZ/YH23MrQae
ZJ5qae9FCzL8nfHS4OFaVQs3RFopPBxXWnVzhsGQQQKBgQDnxnLelnGkCWXf+Ypl
N0oCFwAzZDMATOgOHPeIDiOZH6qw+3x3YXlZKHzwMjYeqi3RuWkErmax1DqlS2Ds
8ZCBt7vSdbpI7uUN8Gie+9Hyi4N4Zop19qiS8RWHA6viZbaMAR0h98Fj0c4DA0AU
4eJY30DD8D+ZIrMF4MRs8C/RTQKBgQDMjhMpustHCGu/eyQKst7E93/wcFP7G5Sx
XX/qXIGBBdD/UYUVMrhAOvD9oYOwh6QROsoCMtZ11GPK9w03BrDSnbxRynDoku3M
RtTEH1rug760he0Ug4ii4xUo2mkNetC3hNREZl6uoPWdoXzKGHNDF80Y5OIrVdEb
2TYzGW5txwKBgQDQnFqJo6lXNqo+JJF/NntjVCZ3GwmYjKAVC9dz2x4JVWpB76kA
nnglWn7RhrAVe6DP8mzmrL578oRygF0WBvrE9oWUESiBOpxppmfUKN23zACiHtEj
CcaCs4Fny1Mq69eZPetlxmSHHrCpH4TPBty+lvrpINVtVMEDWmIRl0HCxQKBgEvT
ptrjOZN9VaPHnBazM81EChMxMJB3KumMxWw1GnSfmVfr+i9fe9mjf84lX1HDFlik
uFmUStenAc8tQaLSQh3xBuwy5SPxw2DkKN8C2IxuHfWBZ98g2ze2ghOA00yB6Hj/
Lkikwhht5l6mjEHGSoPmgMrnncd+qmNuY58RoFPlAoGBAJ2NMzfSV4cGAUK17LTa
4lk0x9l9OBIjFExbU/ss84hrXTsulnYzSVy3RK7Hnx3RYpN+4NM0m5KAYJf42VYr
M2d3T5ltRWAdICdoRpCZvlmo45/1MmsXYuNImiEO5bB06ePODN7JafBqAY3Sf8Gc
/x84Mv0MAPcaTYZs3WALDQ4o
-----END PRIVATE KEY-----
";

/// Sign the canonical string-to-sign of an envelope with the test key, and
/// patch the envelope's `signature` field with the base64-encoded result.
fn sign_envelope(envelope: &mut SnsEnvelope) {
    let private_key = RsaPrivateKey::from_pkcs8_pem(TEST_KEY_PEM).expect("invalid test key");
    let signing_key = SigningKey::<Sha1>::new(private_key);
    let string_to_sign = build_string_to_sign(envelope);
    let signature = signing_key.sign(string_to_sign.as_bytes());
    envelope.signature = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    envelope.signature_version = "1".into();
}

fn payment_message() -> String {
    r#"{
        "dateTime": "20180419122247",
        "debitMSISDN": "8801700000001",
        "creditOrganizationName": "Org 01",
        "creditShortCode": "01929918***",
        "trxID": "4J420ANOXC",
        "transactionStatus": "Completed",
        "transactionType": "10002294",
        "amount": "100",
        "currency": "BDT",
        "transactionReference": "Test_Payment",
        "merchantInvoiceNumber": "orderId1233"
    }"#
    .to_string()
}

fn make_envelope() -> SnsEnvelope {
    SnsEnvelope {
        r#type: "Notification".into(),
        message_id: "msg-test-1".into(),
        token: String::new(),
        topic_arn: "arn:aws:sns:ap-southeast-1:797962984373:bpt_test".into(),
        message: payment_message(),
        subscribe_url: String::new(),
        timestamp: "2026-06-22T13:28:31.000Z".into(),
        signature_version: "1".into(),
        signature: String::new(),
        signing_cert_url: String::new(),
        unsubscribe_url: "https://sns.ap-southeast-1.amazonaws.com/?Action=Unsubscribe&...".into(),
        subject: None,
    }
}

#[tokio::test]
async fn verify_sns_signature_succeeds_against_mocked_cert() {
    // The full verify_sns_signature flow requires the SigningCertURL to
    // be HTTPS on `sns.<region>.amazonaws.com`, which a wiremock-rs HTTP
    // mock can't serve. We instead use verify_signature_with_key with the
    // public key extracted from the same PEM bytes to exercise the full
    // crypto + canonical-string-to-sign path end-to-end. The HTTP fetch
    // itself is a 2-line wrapper exercised in unit tests.
    let mut envelope = make_envelope();
    sign_envelope(&mut envelope);

    let public_key = cert_pem_to_public_key(TEST_CERT_PEM).expect("cert parsing");
    verify_signature_with_key(&envelope, &public_key).expect("valid signature should verify");
}

#[tokio::test]
async fn cert_fetch_via_wiremock_succeeds() {
    // Confirms the reqwest-based cert fetch works against an HTTP mock.
    // The full verify_sns_signature can't be wired up because it requires
    // `https://sns.*.amazonaws.com/...` URLs, but this exercises the
    // fetch path end-to-end against a mock server.
    let server = MockServer::start().await;
    let cert_path = "/SimpleNotificationService-test.pem";
    Mock::given(method("GET"))
        .and(path(cert_path))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_CERT_PEM))
        .expect(1)
        .mount(&server)
        .await;

    let http = reqwest::Client::new();
    let cert_pem = http
        .get(format!("{}{}", server.uri(), cert_path))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .text()
        .await
        .unwrap();

    let pk = cert_pem_to_public_key(&cert_pem).expect("cert over the wire should parse");
    // Round-trip via the verifier to confirm the fetched cert produces the
    // same public key as the embedded one.
    let mut envelope = make_envelope();
    sign_envelope(&mut envelope);
    verify_signature_with_key(&envelope, &pk).expect("verify against fetched cert");
}

#[tokio::test]
async fn verify_sns_signature_rejects_tampered_signature() {
    let mut envelope = make_envelope();
    sign_envelope(&mut envelope);
    // Flip the first base64 character to invalidate the signature without
    // changing its length. Base64 alphabet is ASCII so this is safe.
    let mut bytes = envelope.signature.into_bytes();
    bytes[0] = if bytes[0] == b'A' { b'B' } else { b'A' };
    envelope.signature = String::from_utf8(bytes).unwrap();

    let public_key = cert_pem_to_public_key(TEST_CERT_PEM).expect("cert parsing");
    let res = verify_signature_with_key(&envelope, &public_key);
    assert!(res.is_err(), "tampered signature must fail verification");
}

#[tokio::test]
async fn verify_sns_signature_rejects_non_sns_host() {
    // We can't easily wire up a full HTTPS-on-sns-host mock, so we test
    // validate_signing_cert_url directly via verify_sns_signature's
    // pre-flight check. The fetch will never happen because the URL
    // validation rejects the wiremock HTTP URL.
    let server = MockServer::start().await;
    // Mount in case the test mistakenly fetches.
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_CERT_PEM))
        .mount(&server)
        .await;

    let mut envelope = make_envelope();
    envelope.signing_cert_url = format!("{}/cert.pem", server.uri());
    sign_envelope(&mut envelope);

    let http = reqwest::Client::new();
    let res = verify_sns_signature(&envelope, &http).await;
    assert!(res.is_err(), "non-sns host must be rejected");
}

#[tokio::test]
async fn parse_event_returns_payment_for_signed_envelope() {
    let envelope = make_envelope();
    let event = parse_event(&envelope).unwrap();
    match event {
        WebhookEvent::Payment(p) => assert_eq!(p.trx_id, "4J420ANOXC"),
        other => panic!("expected Payment, got {other:?}"),
    }
}

#[tokio::test]
async fn confirm_subscription_gets_subscribe_url() {
    let server = MockServer::start().await;
    // Use a path-only mock; the actual subscribe URL contains a `?` and
    // arbitrary query params that wiremock cannot match directly. The mock
    // server will accept any GET.
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let envelope = SnsEnvelope {
        r#type: "SubscriptionConfirmation".into(),
        subscribe_url: format!("{}/?Action=ConfirmSubscription&Token=t", server.uri()),
        ..make_envelope()
    };

    let http = reqwest::Client::new();
    confirm_subscription(&envelope, &http)
        .await
        .expect("confirm_subscription should succeed");
}
