//! Approval gate — cơ chế yêu cầu user approve tool call nguy hiểm.
//!
//! Flow:
//! 1. Agent gọi `ApprovalGate::request(req)` — block trên `oneshot::Receiver`.
//! 2. Frontend nhận event `approval:request`, hiện dialog.
//! 3. User click Approve/Reject → frontend invoke `approval:respond`.
//! 4. `ApprovalGate::respond(id, decision)` resolve oneshot.
//! 5. Agent tiếp tục.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, Mutex};
use tokio::time::timeout;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::SecurityError;
use crate::security::PermissionLevel;
use crate::Result;

/// Quyết định của user.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approved,
    Rejected,
}

/// Request approval cho 1 tool call.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ApprovalRequest {
    pub id: String,
    pub tool: String,
    pub input: serde_json::Value,
    pub permission: PermissionLevel,
    pub session_id: Option<String>,
    pub run_id: String,
}

/// Entry nội bộ trong gate.
struct Pending {
    tx: oneshot::Sender<ApprovalDecision>,
    req: ApprovalRequest,
}

/// Approval gate — sync access qua Mutex.
#[derive(Default)]
pub struct ApprovalGate {
    pending: Mutex<HashMap<String, Pending>>,
    timeout_secs: u64,
}

impl ApprovalGate {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            timeout_secs,
        }
    }

    /// Request approval — block cho đến khi user respond hoặc timeout.
    ///
    /// Trả về request (để caller emit event) qua callback `on_request`.
    pub async fn request<F>(&self, req: ApprovalRequest, on_request: F) -> Result<ApprovalDecision>
    where
        F: FnOnce(ApprovalRequest),
    {
        let id = req.id.clone();
        let (tx, rx) = oneshot::channel::<ApprovalDecision>();

        {
            let mut map = self.pending.lock().await;
            map.insert(id.clone(), Pending { tx, req: req.clone() });
        }

        // Emit event cho frontend
        on_request(req);

        // Wait với timeout
        let dur = Duration::from_secs(self.timeout_secs);
        match timeout(dur, rx).await {
            Ok(Ok(decision)) => {
                self.remove(&id).await;
                if decision == ApprovalDecision::Rejected {
                    Err(SecurityError::ApprovalDenied.into())
                } else {
                    Ok(decision)
                }
            }
            Ok(Err(_)) => {
                self.remove(&id).await;
                Err(SecurityError::ApprovalDenied.into())
            }
            Err(_) => {
                self.remove(&id).await;
                Err(SecurityError::ApprovalTimeout(dur).into())
            }
        }
    }

    /// User respond từ frontend.
    pub async fn respond(&self, id: &str, decision: ApprovalDecision) -> Result<()> {
        let mut map = self.pending.lock().await;
        if let Some(pending) = map.remove(id) {
            let _ = pending.tx.send(decision);
            Ok(())
        } else {
            Err(crate::error::NexusError::NotFound(format!(
                "approval request {id} not found (maybe timed out)"
            )))
        }
    }

    /// Lấy danh sách pending requests (cho UI debug).
    pub async fn pending(&self) -> Vec<ApprovalRequest> {
        let map = self.pending.lock().await;
        map.values().map(|p| p.req.clone()).collect()
    }

    async fn remove(&self, id: &str) {
        let mut map = self.pending.lock().await;
        map.remove(id);
    }

    /// Sinh ID mới cho request.
    #[must_use]
    pub fn new_id() -> String {
        Uuid::new_v4().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn approve_resolves() {
        let gate = ApprovalGate::new(10);
        let req = ApprovalRequest {
            id: ApprovalGate::new_id(),
            tool: "write_file".into(),
            input: serde_json::json!({"path": "x.txt"}),
            permission: PermissionLevel::RequiresApproval,
            session_id: None,
            run_id: "r1".into(),
        };
        let id = req.id.clone();
        let gate_clone_id = id.clone();
        let gate_ref = &gate;
        // Spawn requester
        let h = tokio::spawn(async move {
            gate_ref
                .request(req, |_r| {})
                .await
        });
        // Respond
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        gate.respond(&gate_clone_id, ApprovalDecision::Approved).await.unwrap();
        let res = h.await.unwrap();
        assert_eq!(res.unwrap(), ApprovalDecision::Approved);
    }

    #[tokio::test]
    async fn reject_returns_error() {
        let gate = ApprovalGate::new(10);
        let req = ApprovalRequest {
            id: ApprovalGate::new_id(),
            tool: "delete_file".into(),
            input: serde_json::json!({}),
            permission: PermissionLevel::Dangerous,
            session_id: None,
            run_id: "r1".into(),
        };
        let id = req.id.clone();
        let gate_ref = &gate;
        let h = tokio::spawn(async move { gate_ref.request(req, |_| {}).await });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        gate.respond(&id, ApprovalDecision::Rejected).await.unwrap();
        let res = h.await.unwrap();
        assert!(res.is_err());
    }
}
