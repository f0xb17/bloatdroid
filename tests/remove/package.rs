//! Tests for the package identity of a removal.

use bloatdroid::remove::RemovalPackage;

#[test]
fn names_the_package_it_targets() {
    let package = RemovalPackage::new("com.example.app");

    assert_eq!(package.identifier(), "com.example.app");
}

#[test]
fn accepts_a_name_without_a_dot() {
    let package = RemovalPackage::new(String::from("android"));

    assert_eq!(package.identifier(), "android");
}

#[test]
fn keeps_the_package_identity_alone() {
    let package = RemovalPackage::new("com.example.app");
    let same = RemovalPackage::new("com.example.app");
    let other = RemovalPackage::new("com.example.other");
    let copy = package.clone();

    // The value carries the identity only: the removal method and the execution
    // state belong to the request and to its result.
    assert_eq!(copy, package);
    assert_eq!(same, package);
    assert_ne!(other, package);
    assert_eq!(package.identifier(), "com.example.app");
}
