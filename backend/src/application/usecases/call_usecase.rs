//! Call lifecycle use cases (request, response, hangup).

use std::io;

use crate::domain::{CallState, UserId, UserState};
use crate::infrastructure::storage::Storage;
use crate::tcp::messages::{
    CallAcceptedMsg, CallDeclinedMsg, CallNotificationMsg, CallRequest, CallResponseMsg, ErrorMsg,
    HangupMsg, Message,
};

/// Call management use case handler
pub struct CallUseCase {
    storage: Storage,
    logger: logging::Logger,
}

impl CallUseCase {
    pub fn new(storage: Storage, logger: logging::Logger) -> Self {
        CallUseCase { storage, logger }
    }

    /// Handle call request from caller
    pub fn handle_call_request(
        &self,
        caller_id: &UserId,
        req: &CallRequest,
    ) -> io::Result<Option<Message>> {
        self.logger.info(&format!(
            "Received call request from {} to {}",
            caller_id, req.to_user_id
        ));

        // Check if caller is trying to call themselves
        if caller_id == &req.to_user_id {
            self.logger.error("Caller cannot call themselves");
            return Ok(Some(Message::Error(ErrorMsg {
                code: 400,
                message: "Cannot call yourself".to_string(),
            })));
        }

        // Check if callee is available
        let callee_state = self.storage.get_user_state(&req.to_user_id);
        if callee_state != Some(UserState::Available) {
            self.logger.error(&format!(
                "Callee {} is not available for calls",
                req.to_user_id
            ));
            return Ok(Some(Message::Error(ErrorMsg {
                code: 400,
                message: "User not available for calls".to_string(),
            })));
        }

        // Create call
        let call = match self
            .storage
            .create_call(caller_id.clone(), req.to_user_id.clone())
        {
            Ok(c) => c,
            Err(e) => {
                self.logger.error(&format!("Failed to create call: {}", e));
                return Ok(Some(Message::Error(ErrorMsg {
                    code: 500,
                    message: format!("Failed to create call: {}", e),
                })));
            }
        };
        let call_id = call.call_id.clone();

        // Send notification to callee
        let caller = self.storage.get_user(caller_id);
        if let Some(caller_user) = caller {
            let notification = Message::CallNotification(CallNotificationMsg {
                call_id: call_id.clone(),
                from_user_id: caller_id.clone(),
                from_username: caller_user.username.clone(),
            });

            let _ = self.storage.forward_to_user(&req.to_user_id, notification);
        }

        self.logger.info(&format!(
            "Call initiated: {} -> {}",
            caller_id, req.to_user_id
        ));

        Ok(None) // No direct response to caller yet
    }

    /// Handle call response (accept/decline) from callee
    pub fn handle_call_response(
        &self,
        callee_id: &UserId,
        resp: &CallResponseMsg,
    ) -> io::Result<Option<Message>> {
        self.logger.info(&format!(
            "Received call response from {} for call {}: accepted={}",
            callee_id, resp.call_id, resp.accepted
        ));

        let call = match self.storage.get_call(&resp.call_id) {
            Some(c) => c,
            None => return Ok(None),
        };

        if resp.accepted {
            self.accept_call(callee_id, &resp.call_id, &call.caller_id)?;
        } else {
            self.decline_call(callee_id, &resp.call_id, &call.caller_id)?;
        }

        self.logger.info(&format!(
            "Call response processed for call {}: accepted={}",
            resp.call_id, resp.accepted
        ));
        Ok(None)
    }

    /// Accept call and notify caller
    fn accept_call(&self, callee_id: &UserId, call_id: &str, caller_id: &UserId) -> io::Result<()> {
        // Update call state to Active
        let _ = self.storage.update_call_state(call_id, CallState::Active);

        // Get callee info
        let callee_user = self.storage.get_user(callee_id);
        let callee_username = callee_user
            .as_ref()
            .map(|u| u.username.clone())
            .unwrap_or_default();

        // Notify caller
        let accepted_msg = Message::CallAccepted(CallAcceptedMsg {
            call_id: call_id.to_string(),
            peer_user_id: callee_id.clone(),
            peer_username: callee_username,
        });
        let _ = self.storage.forward_to_user(caller_id, accepted_msg);

        self.logger.info(&format!("Call accepted: {}", call_id));
        self.logger.info(&format!(
            "Users {} and {} set to Busy",
            caller_id, callee_id
        ));

        Ok(())
    }

    /// Decline call and notify caller
    fn decline_call(
        &self,
        callee_id: &UserId,
        call_id: &str,
        caller_id: &UserId,
    ) -> io::Result<()> {
        // Remove call
        self.storage.remove_call(call_id);

        // Get callee info
        let callee_user = self.storage.get_user(callee_id);
        let callee_username = callee_user
            .as_ref()
            .map(|u| u.username.clone())
            .unwrap_or_default();

        // Notify caller
        let declined_msg = Message::CallDeclined(CallDeclinedMsg {
            call_id: call_id.to_string(),
            peer_user_id: callee_id.clone(),
            peer_username: callee_username,
        });
        let _ = self.storage.forward_to_user(caller_id, declined_msg);

        self.logger.info(&format!("Call declined: {}", call_id));

        Ok(())
    }

    /// Handle hangup from either party
    pub fn handle_hangup(
        &self,
        user_id: &UserId,
        hangup: &HangupMsg,
    ) -> io::Result<Option<Message>> {
        self.logger.info(&format!(
            "Received hangup from {} for call {}",
            user_id, hangup.call_id
        ));

        if let Some(call) = self.storage.remove_call(&hangup.call_id) {
            let peer_id = if &call.caller_id == user_id {
                call.callee_id.clone()
            } else {
                call.caller_id.clone()
            };

            // Forward hangup to peer
            let _ = self.storage.forward_to_user(
                &peer_id,
                Message::Hangup(HangupMsg {
                    call_id: hangup.call_id.clone(),
                }),
            );

            self.storage
                .broadcast_state_update(user_id, UserState::Available);
            self.storage
                .broadcast_state_update(&peer_id, UserState::Available);

            self.logger.info(&format!(
                "Call ended: {} - both users set to Available",
                hangup.call_id
            ));
        }

        Ok(None)
    }

    /// Cleanup when user disconnects (end active calls)
    pub fn cleanup_user_disconnect(&self, user_id: &UserId) {
        self.logger
            .info(&format!("User {} disconnecting - cleanup calls", user_id));

        // Get active call for this user
        if let Some(call) = self.storage.get_user_active_call(user_id) {
            // Notify the other party
            let peer_id = if &call.caller_id == user_id {
                call.callee_id.clone()
            } else {
                call.caller_id.clone()
            };

            let hangup_msg = Message::Hangup(HangupMsg {
                call_id: call.call_id.clone(),
            });
            let _ = self.storage.forward_to_user(&peer_id, hangup_msg);

            // Remove the call
            self.storage.remove_call(&call.call_id);
        }

        // Disconnect user (handles state broadcast)
        let _ = self.storage.disconnect_user(user_id);
    }
}
