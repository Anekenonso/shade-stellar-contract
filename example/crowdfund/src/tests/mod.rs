pub mod test;
pub mod test_feature_191;
pub mod test_feature_194;
// test_feature_214 targets a balance-based affiliate API (set_affiliate_commission_bps,
// contribute_with_affiliate, affiliate_balance, claim_affiliate_commission, is_affiliate)
// whose implementation was overwritten by the referral-code system in #349. The file was
// never declared here, so it has never compiled or run. Its orphaned AffiliateAccruedEvent
// and AffiliateClaimedEvent declarations remain in lib.rs.
// pub mod test_feature_214;
