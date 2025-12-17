//! Message handler - Orchestrates usecases based on incoming messages

use std::io;

use crate::application::usecases::{AuthUseCase, CallUseCase, SignalingUseCase, UserUseCase};
use crate::domain::UserId;
use crate::infrastructure::storage::Storage;
use crate::tcp::messages::{ErrorMsg, Message};

/// Message orchestrator - delegates to appropriate use cases
pub struct MessageHandler {
    auth_usecase: AuthUseCase,
    call_usecase: CallUseCase,
    signaling_usecase: SignalingUseCase,
    user_usecase: UserUseCase,
}

impl MessageHandler {
    pub fn new(storage: Storage, logger: logging::Logger) -> Self {
        let auth_logger = logger
            .for_component("Auth Usecase")
            .unwrap_or_else(|_| logger.clone());
        let call_logger = logger
            .for_component("Call Usecase")
            .unwrap_or_else(|_| logger.clone());
        let signaling_logger = logger
            .for_component("Signaling Usecase")
            .unwrap_or_else(|_| logger.clone());
        let user_logger = logger
            .for_component("User Usecase")
            .unwrap_or_else(|_| logger.clone());

        MessageHandler {
            auth_usecase: AuthUseCase::new(storage.clone(), auth_logger),
            call_usecase: CallUseCase::new(storage.clone(), call_logger),
            signaling_usecase: SignalingUseCase::new(storage.clone(), signaling_logger),
            user_usecase: UserUseCase::new(storage.clone(), user_logger),
        }
    }

    /// Process incoming message and delegate to appropriate use case
    pub fn process_message(
        &self,
        message: Message,
        authenticated_user_id: Option<&UserId>,
    ) -> io::Result<Option<Message>> {
        match message {
            // Messages not requiring authentication
            Message::RegisterRequest(req) => self.auth_usecase.handle_register(&req).map(Some),
            Message::UserListRequest => self.user_usecase.handle_user_list().map(Some),
            Message::Heartbeat(_) => Ok(None), // Heartbeat handled, no response needed

            // Login must be handled by ClientHandler (needs TCP stream for writer thread)
            Message::LoginRequest(_) => Err(io::Error::other(
                "Login must be handled by ClientHandler for TCP stream management",
            )),

            // Messages requiring authentication
            Message::LogoutRequest(_) => self.require_auth(authenticated_user_id, |user_id| {
                self.auth_usecase.handle_logout(user_id).map(Some)
            }),
            Message::CallRequest(req) => self.require_auth(authenticated_user_id, |caller_id| {
                self.call_usecase.handle_call_request(caller_id, &req)
            }),
            Message::CallResponse(resp) => self.require_auth(authenticated_user_id, |callee_id| {
                self.call_usecase.handle_call_response(callee_id, &resp)
            }),
            Message::SdpOffer(offer) => self.require_auth(authenticated_user_id, |_| {
                self.signaling_usecase.handle_sdp_offer(&offer)
            }),
            Message::SdpAnswer(answer) => self.require_auth(authenticated_user_id, |_| {
                self.signaling_usecase.handle_sdp_answer(&answer)
            }),
            Message::IceCandidate(candidate) => self.require_auth(authenticated_user_id, |_| {
                self.signaling_usecase.handle_ice_candidate(&candidate)
            }),
            Message::Hangup(hangup) => self.require_auth(authenticated_user_id, |user_id| {
                self.call_usecase.handle_hangup(user_id, &hangup)
            }),
            _ => Ok(Some(Message::Error(ErrorMsg {
                code: 400,
                message: format!("Invalid message type from client with type: {:?}", message),
            }))),
        }
    }

    /// Helper to require authentication before processing
    fn require_auth<F>(
        &self,
        authenticated_user_id: Option<&UserId>,
        handler: F,
    ) -> io::Result<Option<Message>>
    where
        F: FnOnce(&UserId) -> io::Result<Option<Message>>,
    {
        match authenticated_user_id {
            Some(user_id) => handler(user_id),
            None => Ok(Some(Message::Error(ErrorMsg {
                code: 401,
                message: "User Not Authenticated".to_string(),
            }))),
        }
    }

    /// Cleanup when user disconnects
    pub fn cleanup_user_disconnect(&self, user_id: &UserId) {
        self.call_usecase.cleanup_user_disconnect(user_id);
    }
}
