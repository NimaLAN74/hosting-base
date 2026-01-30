//! Supported compliance frameworks for framework-aware prompts.
//! Aligned with COMP-AI-MULTIFRAMEWORK-SUPPORT.md (SOC 2, ISO 27001, GDPR, etc.).

use serde::Serialize;

/// One framework offered in the API (id used in request, name for display).
#[derive(Debug, Clone, Serialize)]
pub struct FrameworkItem {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

/// Static list of supported frameworks (Phase 3: first-class).
const FRAMEWORKS: &[FrameworkItem] = &[
    FrameworkItem {
        id: "soc2",
        name: "SOC 2",
        description: "Trust Services Criteria (security, availability, processing integrity, confidentiality, privacy)",
    },
    FrameworkItem {
        id: "iso27001",
        name: "ISO 27001",
        description: "Information security management system (ISMS), Annex A controls",
    },
    FrameworkItem {
        id: "gdpr",
        name: "GDPR",
        description: "EU General Data Protection Regulation (privacy, DPA, breach, rights)",
    },
    FrameworkItem {
        id: "hipaa",
        name: "HIPAA",
        description: "US Health Insurance Portability and Accountability Act (PHI, safeguards)",
    },
    FrameworkItem {
        id: "pci_dss",
        name: "PCI DSS",
        description: "Payment Card Industry Data Security Standard",
    },
    FrameworkItem {
        id: "nist_csf",
        name: "NIST CSF 2.0",
        description: "NIST Cybersecurity Framework (Identify, Protect, Detect, Respond, Recover)",
    },
];

/// Returns all supported frameworks for the API.
pub fn list() -> &'static [FrameworkItem] {
    FRAMEWORKS
}

/// Returns true if `id` is a supported framework id (case-insensitive).
pub fn is_supported(id: &str) -> bool {
    let id = id.trim().to_lowercase();
    if id.is_empty() {
        return true;
    }
    FRAMEWORKS.iter().any(|f| f.id == id)
}

/// Returns a short compliance context prefix for the model prompt (e.g. "Answer in the context of SOC 2 compliance.").
pub fn prompt_prefix(id: &str) -> Option<&'static str> {
    let id = id.trim().to_lowercase();
    if id.is_empty() {
        return None;
    }
    let desc = match id.as_str() {
        "soc2" => "Answer in the context of SOC 2 (Trust Services Criteria) compliance. Be concise; cite control IDs (e.g. CC6.1) where relevant.",
        "iso27001" => "Answer in the context of ISO 27001 (ISMS, Annex A) compliance. Be concise; cite control IDs (e.g. A.9.4.2) where relevant.",
        "gdpr" => "Answer in the context of EU GDPR compliance. Be concise; cite articles where relevant.",
        "hipaa" => "Answer in the context of HIPAA (US healthcare, PHI) compliance. Be concise; cite safeguards where relevant.",
        "pci_dss" => "Answer in the context of PCI DSS compliance. Be concise; cite requirements where relevant.",
        "nist_csf" => "Answer in the context of NIST Cybersecurity Framework (CSF 2.0) compliance. Be concise; cite categories/subcategories where relevant.",
        _ => return None,
    };
    Some(desc)
}
