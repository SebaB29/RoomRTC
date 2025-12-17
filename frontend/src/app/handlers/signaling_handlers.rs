//! WebRTC Signaling Handlers
//!
//! Handles SDP offers, answers, and ICE candidate exchange

use crate::app::state::App;
use crate::events::LogicCommand;
use crate::pages::Page;

impl App {
    /// Handles incoming SDP offer (callee side)
    pub(in crate::app) fn handle_sdp_offer(&mut self, call_id: String, offer: String) {
        let my_name = self.user_context.get_name().unwrap_or("UNKNOWN");

        // Verify we're in the correct room already
        if self.current_page != Page::Room {
            self.logger.warn(&format!(
                "[SIGNALING] Received SDP offer but not in Room page - user: '{}', current_page: {:?}",
                my_name, self.current_page
            ));
        }

        self.logger.info(&format!(
            "[SIGNALING] Generating SDP answer for user '{}' - call_id: {}",
            my_name, call_id
        ));

        // Generate answer from received offer
        // StartConnection will be called automatically after answer is sent
        let _ = self
            .logic_cmd_tx
            .send(LogicCommand::GenerateAnswer { offer_sdp: offer });
    }

    /// Handles incoming SDP answer (caller side)
    pub(in crate::app) fn handle_sdp_answer(&mut self, answer: String) {
        let my_name = self.user_context.get_name().unwrap_or("UNKNOWN");

        self.logger.info(&format!(
            "[SIGNALING] Processing SDP answer for user '{}'",
            my_name
        ));

        // Process the answer to complete connection
        let _ = self
            .logic_cmd_tx
            .send(LogicCommand::ProcessAnswer { answer_sdp: answer });
    }

    /// Handles incoming ICE candidate
    pub(in crate::app) fn handle_ice_candidate(
        &mut self,
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: u32,
    ) {
        // Send to logic thread to add ICE candidate to webrtc_connection
        let _ = self.logic_cmd_tx.send(LogicCommand::AddIceCandidate {
            candidate,
            sdp_mid,
            sdp_mline_index: sdp_mline_index as u16,
        });
    }
}
