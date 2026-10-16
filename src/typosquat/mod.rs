//! Typosquat detection module.
//!
//! Bundles the reference data a typosquat check needs — the lists of popular
//! package names an attacker is most likely to imitate ([`dataset`]) — the
//! independent signals for comparing a candidate name against them
//! (edit distance in [`distance`], lookalike characters in [`homoglyph`],
//! imitated npm scopes in [`scope`], bait-word combinations in
//! [`combosquat`], plus the adjacent-key reason in [`keyboard`]) — and
//! [`scanner::score_package`], which combines them into a single scored
//! result. Candidates are lowercased and npm scoped names split into scope
//! and bare name first ([`normalize`]).

pub mod combosquat;
pub mod dataset;
pub mod distance;
pub mod homoglyph;
pub mod keyboard;
pub mod normalize;
pub mod scanner;
pub mod scope;
