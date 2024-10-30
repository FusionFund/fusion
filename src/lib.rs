// use near_sdk::{log, near};
use near_sdk::{env, log, near, require, AccountId, NearToken, PanicOnDefault, Promise};
use near_sdk::json_types::U64;
use near_sdk::borsh::{self, BorshSerialize, BorshDeserialize};
use near_sdk::store::{LookupMap, IterableMap, IterableSet, Vector};
use near_sdk::BorshStorageKey;

mod dao;
mod kyc;

#[near]
#[derive(BorshStorageKey)]
pub enum Prefix {
    Root,
    Vector,
    LookupSet,
    IterableSet,
    LookupMap,
    IterableMap,
    Nested(String),
}


#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    // total_contributions: u64,
    // contributions: Vec<Contribution>,
    // campaigns: Vec<Campaign>,
    campaigns : IterableMap<u64, Campaign>,
    users: IterableMap<AccountId, UserProfile>,
    next_campaign_id: u64,
    pub loan_requests: IterableMap<u64, LoanRequest>,
    pub loans: IterableMap<u64, Loan>,
    pub next_loan_request_id: u64,
    pub next_loan_id: u64,
    treasury: u64,
    proposals: IterableMap<u64, dao::Proposal>,
    trusted_members: Vector<AccountId>,
    proposal_count: u64,
    pub verified_users: IterableSet<AccountId>, // Set of verified users
    pub banned_users: IterableSet<AccountId>, 
    pub withdrawal_logs: IterableMap<u64, Vec<WithdrawalLog>>, // Each campaign's logs
    pub campaign_stats: IterableMap<u64, CampaignStats>,
    // crowdfunding_end_time: U64,
    // project_creator: AccountId,
    // claimed: bool,
}



#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct LoanRequest {
    pub borrower: AccountId,
    pub amount: u64,
    pub interest_rate: u8, // interest rate as a percentage
    pub duration: U64, // loan duration in seconds
    pub fulfilled: bool,
}


#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Loan {
    pub loan_id: u64,
    pub borrower: AccountId,
    pub lender: AccountId,
    pub amount: u64,
    pub interest_rate: u8,
    pub duration: U64,
    pub start_time: U64,
    pub repaid: bool,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct WithdrawalLog {
    pub campaign_id: u64,
    pub withdrawn_by: AccountId,
    pub amount: u64,
    pub timestamp: U64,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Default)]
pub struct CampaignStats {
    pub total_funds: u64,
    pub total_withdrawn: u64,
    pub number_of_withdrawals: u64,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Campaign {
    pub creator: AccountId,
    pub total_contributions: u64,
    pub contributions: Vec<Contribution>,
    pub crowdfunding_end_time: U64,
    pub claimed: bool,
    pub amount_required : u64,
    pub title : String,
    pub description : String,
    pub images : String,
    pub campaign_code : String
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct UserProfile {
    pub username: String,
    pub bio: Option<String>,
    pub kyc_verified: bool,
    pub contributions: Vec<u64>, // Campaign IDs where the user has contributed
    pub created_campaigns: Vec<u64>, // Campaign IDs created by the user
}

// Defining a very simple structure for a contribution
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Contribution {
    pub contributor: AccountId,
    pub amount: u64,
}



// Implement the contract structure
#[near]
impl Contract {
    #[init]
    #[private] // only callable by the contract's account
    pub fn init() -> Self {
        Self {
            // campaigns: Vec::new(),
            campaigns : IterableMap::new(Prefix::IterableMap), 
            users: IterableMap::new(Prefix::IterableMap),
            next_campaign_id: 0,
            loan_requests : IterableMap::new(Prefix::IterableMap),
            loans : IterableMap::new(Prefix::IterableMap),
            next_loan_id : 0,
            next_loan_request_id : 0,
            treasury : 0,
            proposals : IterableMap::new(Prefix::IterableMap),
            trusted_members : Vector::new(Prefix::Vector),
            proposal_count : 0,
            verified_users : IterableSet::new(Prefix::IterableSet),
            banned_users : IterableSet::new(Prefix::IterableSet),
            withdrawal_logs : IterableMap::new(Prefix::IterableMap),
            campaign_stats : IterableMap::new(Prefix::IterableMap)
        }
    }

    #[payable]
    pub fn create_loan_request(&mut self, amount: u64, interest_rate: u8, duration: U64) -> u64 {
        let borrower = env::predecessor_account_id();
        let loan_request_id = self.next_loan_request_id;
        
        // Create loan request
        let loan_request = LoanRequest {
            borrower,
            amount,
            interest_rate,
            duration,
            fulfilled: false,
        };
        
        self.loan_requests.insert(loan_request_id, loan_request);
        self.next_loan_request_id += 1;
        
        loan_request_id
    }

    #[payable]
    pub fn accept_loan_request(&mut self, loan_request_id: u64) -> u64 {
        let lender = env::predecessor_account_id();
        let loan_request = self.loan_requests.get_mut(&loan_request_id).expect("Loan request not found");
        
        // Ensure loan request is open and not yet fulfilled
        require!(!loan_request.fulfilled, "Loan request is already fulfilled");
        require!(env::attached_deposit() >= NearToken::from_yoctonear(loan_request.amount.into()), "Insufficient deposit to fulfill loan request");

        // Mark the loan request as fulfilled
        loan_request.fulfilled = true;
        // self.loan_requests.insert(loan_request_id, loan_request);
        
        // Create the loan agreement
        let loan_id = self.next_loan_id;
        let loan = Loan {
            loan_id,
            borrower: loan_request.borrower.clone(),
            lender: lender.clone(),
            amount: loan_request.amount,
            interest_rate: loan_request.interest_rate,
            duration: loan_request.duration,
            start_time: U64::from(env::block_timestamp()),
            repaid: false,
        };
        
        self.loans.insert(loan_id, loan);
        self.next_loan_id += 1;

        loan_id
    }

    #[payable]
    pub fn repay_loan(&mut self, loan_id: u64) {
        let borrower = env::predecessor_account_id();
        let loan = self.loans.get_mut(&loan_id).expect("Loan not found");
        
        // Ensure the caller is the borrower and the loan has not been repaid
        require!(loan.borrower == borrower, "Only borrower can repay this loan");
        require!(!loan.repaid, "Loan has already been repaid");

        // Calculate repayment amount (principal + interest)
        let interest_amount = loan.amount * loan.interest_rate as u64 / 100;
        let total_repayment = loan.amount + interest_amount;

        require!(
            env::attached_deposit() >= NearToken::from_yoctonear(total_repayment.into()),
            "Insufficient amount to repay loan"
        );

        // Mark loan as repaid and transfer funds to lender
        loan.repaid = true;
        // self.loans.insert(loan_id, &loan);

        // Transfer funds to lender
        Promise::new(loan.lender.clone()).transfer(NearToken::from_yoctonear(total_repayment.into()));
    }

    pub fn get_loan_request(&self, loan_request_id: u64) -> LoanRequest {
        self.loan_requests.get(&loan_request_id).unwrap().clone()
    }
    
    pub fn get_all_loan_requests(&self) -> Vec<(&u64, &LoanRequest)> {
        self.loan_requests.iter().collect()
    }

    pub fn get_loan(&self, loan_id: u64) -> Loan {
        self.loans.get(&loan_id).unwrap().clone()
    }

    pub fn get_all_loans(&self, from_index: i32, limit: i32) -> Vec<(&u64, &Loan)> {
        self.loans.iter().skip(from_index as usize).take(limit as usize).collect()
    }

    pub fn do_i_exists(&self) -> bool {
        self.users.contains_key(&env::predecessor_account_id())
    }

    pub fn user_exists(&self, account_id : AccountId) -> bool {
        self.users.contains_key(&account_id)
    }

    // Campaigns

    pub fn create_campaign(&mut self, end_time: U64, title : String, description : String, images : String, amount_required : u64, campaign_code : String) {
        let creator = env::predecessor_account_id();
        let profile = self.users.get_mut(&creator).expect("User profile not found");
        if amount_required > 5000000 {
            // Ensure the user is KYC verified
            require!(profile.kyc_verified, "KYC verification required to create a campaign");
        }
        self.campaigns.insert(self.next_campaign_id, Campaign {
            creator: creator.clone(),
            total_contributions: 0,
            contributions: Vec::new(),
            crowdfunding_end_time: end_time,
            claimed: false,
            amount_required : amount_required,
            title : title,
            description : description.to_string(),
            images : images.to_string(),
            campaign_code : campaign_code
        },);
        profile.created_campaigns.push(self.next_campaign_id);
        // self.users.insert(creator, profile);


    }

    #[payable]
    pub fn create_profile(&mut self, username: String, bio: Option<String>) {
        let account_id = env::predecessor_account_id();
        
        // Ensure the profile doesn't already exist
        require!(
            !self.users.contains_key(&account_id),
            "Profile already exists"
        );
        
        let profile = UserProfile {
            username,
            bio,
            kyc_verified: false,
            contributions: vec![],
            created_campaigns: vec![],
        };
        
        self.users.insert(account_id, profile);
    }

    #[private] 
    pub fn verify_kyc(&mut self, user_id: AccountId) {
        let profile = self.users.get_mut(&user_id).expect("User profile not found");
        
        // Update KYC status
        profile.kyc_verified = true;
    }

    #[private] 
    pub fn remove_profile(&mut self, user_id: AccountId) {
        self.users.remove(&user_id);
    }

    pub fn update_profile(&mut self, new_bio: Option<String>) {
        let account_id = env::predecessor_account_id();
        let profile = self.users.get_mut(&account_id).expect("User profile not found");

        // Update profile details
        profile.bio = new_bio;     
    }

    pub fn get_user_profile(&self, user_id: AccountId) -> UserProfile {
        self.users.get(&user_id).unwrap().clone()
    }

    #[payable]
    pub fn contribute(&mut self, campaign_id : u64) {
        // require!(campaign_index < self.campaigns.len(), "Campaign does not exist");
        require!(self.campaigns.contains_key(&campaign_id), "Campaign does not exist");
        
        
        // Get the amount contributed
        let amount = env::attached_deposit();
        let contributor = env::predecessor_account_id();

        let profile = self.users.get_mut(&contributor).expect("User profile not found");

        // let campaign = &mut self.campaigns[campaign_index];
        let campaign = self.campaigns.get_mut(&campaign_id).unwrap();

        // Assert the crowdfunding is still ongoing
        require!(env::block_timestamp() < campaign.crowdfunding_end_time.into(), "Crowdfunding has ended");

        // Record the contribution
        campaign.contributions.push(Contribution {
            contributor,
            amount: amount.as_near() as u64,
        });

        // Update the total contributions
        campaign.total_contributions += amount.as_near() as u64;

        if !profile.contributions.contains(&campaign_id) {
            profile.contributions.push(campaign_id);
        }
    }

   

    pub fn withdraw(&mut self, campaign_id : u64) -> Promise {
        // require!(campaign_index < self.campaigns.len(), "Campaign does not exist");
        require!(self.campaigns.contains_key(&campaign_id), "Campaign does not exist");

        // let campaign = &mut self.campaigns[campaign_index];
        let campaign = self.campaigns.get_mut(&campaign_id).unwrap();

        require!(env::block_timestamp() > campaign.crowdfunding_end_time.into(), "Crowdfunding has not ended yet");
        require!(!campaign.claimed, "Funds have already been claimed");

        require!(campaign.total_contributions >= campaign.amount_required, "Campaign has not reached its funding goal");

        campaign.claimed = true;

        // Transfer total contributions to the project creator
        let promise = Promise::new(campaign.creator.clone()).transfer(NearToken::from_yoctonear(campaign.total_contributions.into()));
        // self.campaigns.insert(campaign_id, campaign.clone()); // Update campaign state
        promise
    }

    #[payable]
    pub fn withdraw_funds(&mut self, campaign_id: u64, recipient: AccountId) {
        let campaign = self.campaigns.get_mut(&campaign_id).expect("Campaign not found");

        // This ensure sthat only the campaign creator can withdraw and the goal has been met.
        require!(
            env::predecessor_account_id() == campaign.creator,
            "Only the campaign creator can withdraw"
        );
        require!(
            campaign.total_contributions >= campaign.amount_required,
            "Funding goal has not been met"
        );

        let amount_to_withdraw = campaign.total_contributions;
        campaign.total_contributions = 0; //NearToken::from_yoctonear(0); // Reset amount after withdrawal

        // Update campaign state
        // self.campaigns.insert(campaign_id, campaign);

        // Transfer funds to the recipient
        Promise::new(recipient.clone()).transfer(NearToken::from_yoctonear(amount_to_withdraw.into()));


        // let logs = self.withdrawal_logs.get(&campaign_id).unwrap();
        
        // logs.push(WithdrawalLog {
        //     campaign_id,
        //     withdrawn_by: env::predecessor_account_id(),
        //     amount: amount_to_withdraw,
        //     timestamp: U64::from(env::block_timestamp()),
        // },);
        // self.withdrawal_logs.insert(campaign_id, logs.clone());

        // Update campaign stats
        let stats = self.campaign_stats.get_mut(&campaign_id).unwrap();
        stats.total_withdrawn += amount_to_withdraw;
        stats.number_of_withdrawals += 1;
        // self.campaign_stats.insert(campaign_id, stats.clone());
    }

    pub fn get_campaign(&self, campaign_id: u64) -> Campaign {
        require!(self.campaigns.contains_key(&campaign_id), "Campaign does not exist");
        self.campaigns.get(&campaign_id).unwrap().clone() // Get the campaign
    }

    pub fn get_all_campaigns_deprecated(&self) -> Vec<&Campaign> {
        self.campaigns.iter().map(|(_, campaign)| campaign).collect() // Collect all campaigns
    }

    pub fn get_all_the_campaign_deprecated(&self) -> Vec<(&u64, &Campaign)> {
        self.campaigns.iter().collect()
    }

    pub fn get_all_campaigns(&self, from_index: i32, limit: i32) -> Vec<(&u64, &Campaign)> {
        self.campaigns
            .iter()
            .skip(from_index as usize)
            .take(limit as usize)
            .collect()
    }

    pub fn get_campaign_status(&self, campaign_id: u64) -> String {
        require!(self.campaigns.contains_key(&campaign_id), "Campaign does not exist");
    
        let campaign = self.campaigns.get(&campaign_id).unwrap();
        let current_time = env::block_timestamp();
    
        if campaign.claimed {
            "Funds claimed".to_string()
        } else if current_time > campaign.crowdfunding_end_time.into() {
            "Crowdfunding ended".to_string()
        } else if campaign.total_contributions >= campaign.amount_required {
            "Funding goal reached".to_string()
        } else {
            "Crowdfunding active".to_string()
        }
    }

    pub fn get_campaign_contributions(&self, campaign_id: u64) -> Vec<Contribution> {
        require!(self.campaigns.contains_key(&campaign_id), "Campaign does not exist");
        self.campaigns.get(&campaign_id).unwrap().contributions.clone()
    }

    pub fn cancel_campaign(&mut self, campaign_id: u64) {
        let campaign = self.campaigns.get(&campaign_id).expect("Campaign does not exist");
    
        require!(env::predecessor_account_id() == campaign.creator, "Only the creator can cancel the campaign");
        require!(env::block_timestamp() < campaign.crowdfunding_end_time.into(), "Cannot cancel after the end time");
    
        // Refund contributions amount back to each contributor
        for contribution in &campaign.contributions {
            Promise::new(contribution.contributor.clone()).transfer(NearToken::from_yoctonear(campaign.total_contributions.into()));
        }
    
        self.campaigns.remove(&campaign_id);
    }

    // pub fn get_user_campaigns(&self, user_id: AccountId) -> Vec<Campaign> {
    //     self.campaigns.iter().filter(|(_, campaign)| campaign.creator == user_id).map(|(_, campaign)| campaign).collect()
    // }

    // pub fn extend_campaign(&mut self, campaign_id: u64, additional_time: U64) {
    //     let mut campaign = self.campaigns.get(&campaign_id).expect("Campaign does not exist");
    
    //     require!(env::predecessor_account_id() == campaign.creator, "Only the creator can extend the campaign");
    //     require!(env::block_timestamp() < campaign.crowdfunding_end_time.into(), "Campaign has already ended");
    
    //     campaign.crowdfunding_end_time += additional_time;
    //     let endtime: U64 = U64::from(campaign.crowdfunding_end_time);
    //     let addtional = U64::from(additional_time);
    //     let newtime : u64 = endtime.clone() + addtional.clone();
    //     self.campaigns.insert(&campaign_id, &campaign);
    // }

    pub fn modify_funding_goal(&mut self, campaign_id: u64, new_goal: u64) {
        let campaign = self.campaigns.get_mut(&campaign_id).expect("Campaign does not exist");
    
        require!(env::predecessor_account_id() == campaign.creator, "Only the creator can modify the funding goal");
        require!(campaign.contributions.is_empty(), "Cannot modify the goal after contributions");
    
        campaign.amount_required = new_goal;
    }

    pub fn refund_contributors(&mut self, campaign_id: u64) {
        let campaign = self.campaigns.get(&campaign_id).expect("Campaign does not exist");
    
        require!(env::block_timestamp() > campaign.crowdfunding_end_time.into(), "Campaign is still active");
        require!(campaign.total_contributions < campaign.amount_required, "Funding goal met; cannot refund");
    
        for contribution in &campaign.contributions {
            Promise::new(contribution.contributor.clone()).transfer(NearToken::from_yoctonear(campaign.total_contributions.into()));
        }
    
        self.campaigns.remove(&campaign_id);
    }

    pub fn get_user_total_contributions(&self, user_id: AccountId) -> u64 {
        self.campaigns.iter()
            .flat_map(|(_, campaign)| campaign.contributions.iter())
            .filter(|contribution| contribution.contributor == user_id)
            .map(|contribution| contribution.amount)
            .sum()
    }

    pub fn get_user_contribution_to_campaign(&self, campaign_id: u64, user_id: AccountId) -> u64 {
        let campaign = self.campaigns.get(&campaign_id).expect("Campaign does not exist");
        campaign.contributions.iter().filter(|c| c.contributor == user_id).map(|c| c.amount).sum()
    }
    
    
}


