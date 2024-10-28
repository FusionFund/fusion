use crate::*;

use near_sdk::{env, AccountId};

#[near]
impl Contract {

    pub fn verify_user(&mut self, user: AccountId) {
        assert!(!self.banned_users.contains(&user), "User is banned.");
        self.verified_users.insert(user);
    }

    // pub fn ban_user(&mut self, user: AccountId) {
    //     self.banned_users.insert(user);
    //     self.verified_users.remove(&user.clone()); // Remove from verified if banned
    // }

    pub fn unban_user(&mut self, user: AccountId) {
        self.banned_users.remove(&user);
    }
}

