use super::fc_sanity_check;
use std::io::Write;

fn package_archive(paths: &[&str]) -> tempfile::NamedTempFile {
    let mut archive_file = tempfile::NamedTempFile::new().unwrap();
    {
        let mut archive = zip::ZipWriter::new(archive_file.as_file_mut());
        for path in paths {
            archive.start_file(*path, zip::write::FileOptions::default()).unwrap();
            archive.write_all(b"{}").unwrap();
        }
        archive.finish().unwrap();
    }
    archive_file
}

#[test]
fn script_package_does_not_request_native_approval() {
    let archive = package_archive(&["mods/Test/mod.json"]);
    assert!(fc_sanity_check(&archive.as_file(), |_| panic!()).is_ok());
}

#[test]
fn invalid_package_does_not_request_native_approval() {
    let archive = package_archive(&["plugins/Test.dll"]);
    assert!(fc_sanity_check(&archive.as_file(), |_| panic!()).is_err());
}

#[test]
fn native_package_requests_one_approval_for_all_plugins() {
    let archive = package_archive(&["mods/Test/mod.json", "plugins/Body.dll", "plugins/Shadow.dll"]);
    let mut approvals = 0;
    assert!(fc_sanity_check(&archive.as_file(), |paths| {
        approvals += 1;
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|path| path.ends_with("Body.dll")));
        assert!(paths.iter().any(|path| path.ends_with("Shadow.dll")));
        true
    }).is_ok());
    assert_eq!(approvals, 1);
}

#[test]
fn denied_native_package_does_not_extract_files() {
    let archive = package_archive(&["mods/Test/mod.json", "plugins/Body.dll"]);
    let target = tempfile::tempdir().unwrap();
    let result = thermite::core::manage::install_with_sanity(
        "Publisher-NativePlugin-1.0.0", archive.as_file(), target.path(),
        |input| fc_sanity_check(input, |_| false),
    );
    assert!(result.is_err());
    assert!(std::fs::read_dir(target.path()).unwrap().next().is_none());
}

#[test]
fn approved_native_package_can_be_uninstalled_and_reinstalled() {
    let archive = package_archive(&["mods/Test/mod.json", "plugins/Body.dll"]);
    let target = tempfile::tempdir().unwrap();
    let installed = thermite::core::manage::install_with_sanity(
        "Publisher-NativePlugin-1.0.0", archive.as_file(), target.path(),
        |input| fc_sanity_check(input, |_| true),
    ).unwrap();
    assert!(installed.join("plugins/Body.dll").is_file());
    assert!(installed.join("mods/Test/mod.json").is_file());
    assert!(installed.canonicalize().unwrap().starts_with(target.path().canonicalize().unwrap()));
    std::fs::remove_dir_all(&installed).unwrap();
    let reinstalled = thermite::core::manage::install_with_sanity(
        "Publisher-NativePlugin-1.0.0", archive.as_file(), target.path(),
        |input| fc_sanity_check(input, |_| true),
    ).unwrap();
    assert!(reinstalled.join("plugins/Body.dll").is_file());
}
