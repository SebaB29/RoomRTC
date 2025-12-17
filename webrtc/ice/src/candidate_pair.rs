//! ICE candidate pair management.
//!
//! This module provides functionality for creating and managing pairs of
//! local and remote ICE candidates with priority calculation.

use crate::candidate::Candidate;

/// Represents a pair of local and remote candidates for connectivity checking.
#[derive(Debug, Clone)]
pub struct CandidatePair {
    pub local: Candidate,
    pub remote: Candidate,
    pub priority: u64,
}

impl CandidatePair {
    /// Creates a new candidate pair with calculated priority.
    ///
    /// # Arguments
    /// * `local` - The local candidate
    /// * `remote` - The remote candidate
    ///
    /// # Returns
    /// A new `CandidatePair` instance with priority calculated according to RFC 5245
    pub fn new(local: Candidate, remote: Candidate) -> Self {
        let priority = Self::calculate_priority(local.priority, remote.priority);

        Self {
            local,
            remote,
            priority,
        }
    }

    /// Calculates the priority for a candidate pair.
    ///
    /// According to RFC 5245:
    /// pair priority = 2^32 * MIN(G,D) + 2 * MAX(G,D) + (G>D?1:0)
    ///
    /// # Arguments
    /// * `g` - Priority of controlling agent's candidate
    /// * `d` - Priority of controlled agent's candidate
    fn calculate_priority(g: u32, d: u32) -> u64 {
        let min = g.min(d) as u64;
        let max = g.max(d) as u64;
        let g_greater = if g > d { 1u64 } else { 0u64 };

        (1u64 << 32) * min + 2 * max + g_greater
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate_type::CandidateType;
    use std::net::{IpAddr, Ipv4Addr};

    fn create_test_candidate(priority: u32, port: u16) -> Candidate {
        Candidate {
            foundation: "test".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority,
            address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        }
    }

    #[test]
    fn test_new_creates_pair_with_candidates() {
        let local = create_test_candidate(1000, 8080);
        let remote = create_test_candidate(2000, 9090);

        let pair = CandidatePair::new(local.clone(), remote.clone());

        assert_eq!(pair.local.priority, 1000);
        assert_eq!(pair.remote.priority, 2000);
        assert_eq!(pair.local.port, 8080);
        assert_eq!(pair.remote.port, 9090);
    }

    #[test]
    fn test_new_calculates_priority() {
        let local = create_test_candidate(1000, 8080);
        let remote = create_test_candidate(2000, 9090);

        let pair = CandidatePair::new(local, remote);

        // Verificar que la prioridad no es cero y fue calculada
        assert!(pair.priority > 0);
    }

    #[test]
    fn test_calculate_priority_when_g_greater_than_d() {
        let g = 2000u32;
        let d = 1000u32;

        let priority = CandidatePair::calculate_priority(g, d);

        // pair priority = 2^32 * MIN(G,D) + 2 * MAX(G,D) + (G>D?1:0)
        // = 2^32 * 1000 + 2 * 2000 + 1
        let expected = (1u64 << 32) * 1000 + 2 * 2000 + 1;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_calculate_priority_when_d_greater_than_g() {
        let g = 1000u32;
        let d = 2000u32;

        let priority = CandidatePair::calculate_priority(g, d);

        // pair priority = 2^32 * MIN(G,D) + 2 * MAX(G,D) + (G>D?1:0)
        // = 2^32 * 1000 + 2 * 2000 + 0
        let expected = (1u64 << 32) * 1000 + 2 * 2000;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_calculate_priority_when_g_equals_d() {
        let g = 1500u32;
        let d = 1500u32;

        let priority = CandidatePair::calculate_priority(g, d);

        // pair priority = 2^32 * MIN(G,D) + 2 * MAX(G,D) + (G>D?1:0)
        // = 2^32 * 1500 + 2 * 1500 + 0
        let expected = (1u64 << 32) * 1500 + 2 * 1500;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_calculate_priority_with_zero_values() {
        let g = 0u32;
        let d = 0u32;

        let priority = CandidatePair::calculate_priority(g, d);

        assert_eq!(priority, 0);
    }

    #[test]
    fn test_calculate_priority_with_large_values() {
        let g = 1_000_000_000u32;
        let d = 1_000_000_000u32;

        let priority = CandidatePair::calculate_priority(g, d);

        let expected = (1u64 << 32) * 1_000_000_000u64 + 2 * 1_000_000_000u64;
        assert_eq!(priority, expected);
        assert!(priority > 0);
    }

    #[test]
    fn test_calculate_priority_with_one_zero() {
        let g = 5000u32;
        let d = 0u32;

        let priority = CandidatePair::calculate_priority(g, d);

        let expected = 2 * 5000 + 1;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_calculate_priority_asymmetry() {
        let priority1 = CandidatePair::calculate_priority(1000, 2000);
        let priority2 = CandidatePair::calculate_priority(2000, 1000);

        assert_ne!(priority1, priority2);
        assert_eq!(priority1 + 1, priority2);
    }

    #[test]
    fn test_calculate_priority_min_max_components() {
        let g = 3000u32;
        let d = 1000u32;

        let priority = CandidatePair::calculate_priority(g, d);

        let min_component = (1u64 << 32) * 1000;
        let max_component = 2 * 3000;
        let g_greater_component = 1;

        assert_eq!(
            priority,
            min_component + max_component + g_greater_component
        );
    }

    #[test]
    fn test_pair_priority_increases_with_candidate_priorities() {
        let pair1 = CandidatePair::new(
            create_test_candidate(100, 8080),
            create_test_candidate(100, 9090),
        );

        let pair2 = CandidatePair::new(
            create_test_candidate(200, 8080),
            create_test_candidate(200, 9090),
        );

        assert!(pair2.priority > pair1.priority);
    }

    #[test]
    fn test_new_with_identical_candidates() {
        let candidate = create_test_candidate(1500, 8080);

        let pair = CandidatePair::new(candidate.clone(), candidate.clone());

        assert_eq!(pair.local.priority, pair.remote.priority);
        assert_eq!(pair.local.port, pair.remote.port);
    }

    #[test]
    fn test_clone_trait() {
        let local = create_test_candidate(1000, 8080);
        let remote = create_test_candidate(2000, 9090);
        let pair = CandidatePair::new(local, remote);

        let cloned = pair.clone();

        assert_eq!(pair.priority, cloned.priority);
        assert_eq!(pair.local.priority, cloned.local.priority);
        assert_eq!(pair.remote.priority, cloned.remote.priority);
    }

    #[test]
    fn test_debug_trait() {
        let local = create_test_candidate(1000, 8080);
        let remote = create_test_candidate(2000, 9090);
        let pair = CandidatePair::new(local, remote);

        let debug_output = format!("{:?}", pair);

        assert!(debug_output.contains("CandidatePair"));
        assert!(debug_output.contains("priority"));
    }

    #[test]
    fn test_calculate_priority_large_values() {
        let g = 2_147_483_647u32;
        let d = 2_147_483_646u32;

        let priority = CandidatePair::calculate_priority(g, d);

        assert!(priority > 0);
        let expected = (1u64 << 32) * (d as u64) + 2 * (g as u64) + 1;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_priority_formula_components_separately() {
        let g = 1234u32;
        let d = 5678u32;

        let priority = CandidatePair::calculate_priority(g, d);

        let min = g.min(d) as u64;
        let max = g.max(d) as u64;
        let g_greater = if g > d { 1u64 } else { 0u64 };

        let manual_calc = (1u64 << 32) * min + 2 * max + g_greater;

        assert_eq!(priority, manual_calc);
    }

    #[test]
    fn test_priority_ordering_different_pairs() {
        let pair_low = CandidatePair::new(
            create_test_candidate(100, 8080),
            create_test_candidate(200, 9090),
        );

        let pair_medium = CandidatePair::new(
            create_test_candidate(500, 8080),
            create_test_candidate(600, 9090),
        );

        let pair_high = CandidatePair::new(
            create_test_candidate(1000, 8080),
            create_test_candidate(1100, 9090),
        );

        assert!(pair_low.priority < pair_medium.priority);
        assert!(pair_medium.priority < pair_high.priority);
    }

    #[test]
    fn test_calculate_priority_boundary_values() {
        let priority1 = CandidatePair::calculate_priority(1, 1);
        assert_eq!(priority1, (1u64 << 32) + 2);

        let priority2 = CandidatePair::calculate_priority(100, 101);
        let expected = (1u64 << 32) * 100 + 2 * 101;
        assert_eq!(priority2, expected);
    }
}
