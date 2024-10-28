use crate::*;

use near_sdk::{env, AccountId};



// Struct to store a proposal
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Proposal {
    id: u64,
    proposer: AccountId,
    description: String,
    votes_for: u64,
    votes_against: u64,
    executed: bool,
}

#[near]
impl Contract {

    
    pub fn add_trusted_member(&mut self, member_id: AccountId) {
        assert!(env::signer_account_id() == env::current_account_id(), "Only the DAO can add members.");
        self.trusted_members.push(member_id);
    }


    pub fn is_a_trusted_member(&self, member_id: &AccountId) -> bool {
        self.trusted_members.iter().any(|m| m == member_id)
    }

    pub fn create_proposal(&mut self, description: String) {
        assert!(self.is_a_trusted_member(&env::signer_account_id()), "Only trusted members can create proposals.");

        let proposal = Proposal {
            id: self.proposal_count,
            proposer: env::signer_account_id(),
            description,
            votes_for: 0,
            votes_against: 0,
            executed: false,
        };

        self.proposals.insert(self.proposal_count, proposal);
        self.proposal_count += 1;
    }

    pub fn vote(&mut self, proposal_id: u64, support: bool) {

        assert!(self.is_a_trusted_member(&env::signer_account_id()), "Only trusted members can vote.");

        let proposal = self.proposals.get_mut(&proposal_id).expect("Proposal not found.");

        
        assert!(!proposal.executed, "Proposal has already been executed.");

        if support {
            proposal.votes_for += 1;
        } else {
            proposal.votes_against += 1;
        }

        // self.proposals.insert(proposal_id, proposal);
    }

    pub fn execute_proposal(&mut self, proposal_id: u64) {
        let proposal = self.proposals.get_mut(&proposal_id).expect("Proposal not found.");
        
        let total_votes = proposal.votes_for + proposal.votes_against;
        let required_votes = self.trusted_members.len() / 2; // Majority rule

        assert!(proposal.votes_for > required_votes as u64, "Proposal did not pass.");
        assert!(!proposal.executed, "Proposal already executed.");

        proposal.executed = true;
        // self.proposals.insert(&proposal_id, &proposal);

        // Execution 
        // .........
    }

    #[payable]
    pub fn contribute_to_treasury(&mut self) {
        let amount = env::attached_deposit();
        self.treasury += amount.as_near() as u64;
    }

    pub fn get_proposal(&self, proposal_id: u64) -> Proposal {
        self.proposals.get(&proposal_id).unwrap().clone()
    }

    // 8. View All Proposals
    pub fn get_all_proposals(&self) -> Vec<&Proposal> {
        self.proposals.values().collect()
    }

}



