//! SDP and ICE candidate forwarding use cases.

use std::io;

use crate::infrastructure::storage::Storage;
use crate::tcp::messages::{IceCandidateMsg, Message, SdpAnswerMsg, SdpOfferMsg};

/// Signaling (SDP/ICE) use case handler
pub struct SignalingUseCase {
    storage: Storage,
    logger: logging::Logger,
}

impl SignalingUseCase {
    pub fn new(storage: Storage, logger: logging::Logger) -> Self {
        SignalingUseCase { storage, logger }
    }

    /// Handle SDP offer and forward to peer
    pub fn handle_sdp_offer(&self, offer: &SdpOfferMsg) -> io::Result<Option<Message>> {
        self.logger.info(&format!(
            "Forwarding SDP offer from {} to {} for call {}. SDP offer details: {:?}",
            offer.from_user_id, offer.to_user_id, offer.call_id, offer.sdp
        ));

        let _ = self
            .storage
            .forward_to_user(&offer.to_user_id, Message::SdpOffer(offer.clone()));

        self.logger.info("SDP offer forwarded successfully");
        Ok(None)
    }

    /// Handle SDP answer and forward to peer
    pub fn handle_sdp_answer(&self, answer: &SdpAnswerMsg) -> io::Result<Option<Message>> {
        self.logger.info(&format!(
            "Forwarding SDP answer from {} to {} for call {}. SDP answer details: {:?}",
            answer.from_user_id, answer.to_user_id, answer.call_id, answer.sdp
        ));

        let _ = self
            .storage
            .forward_to_user(&answer.to_user_id, Message::SdpAnswer(answer.clone()));

        self.logger.info("SDP answer forwarded successfully");
        Ok(None)
    }

    /// Handle ICE candidate and forward to peer
    pub fn handle_ice_candidate(&self, candidate: &IceCandidateMsg) -> io::Result<Option<Message>> {
        self.logger.info(&format!(
            "Forwarding ICE candidate from {} to {} for call {}. Candidate details: {:?}",
            candidate.from_user_id, candidate.to_user_id, candidate.call_id, candidate.candidate
        ));

        let _ = self.storage.forward_to_user(
            &candidate.to_user_id,
            Message::IceCandidate(candidate.clone()),
        );

        self.logger.info("ICE candidate forwarded successfully");
        Ok(None)
    }
}
