use code_m::gameplay::judgment::{Judgment, JudgmentWindows};

#[test]
fn test_judgment_windows_classify_offset_symmetrically() {
    let windows = JudgmentWindows::new(30, 60, 90);

    assert_eq!(windows.classify_offset(0), Judgment::Perfect);
    assert_eq!(windows.classify_offset(30), Judgment::Perfect);
    assert_eq!(windows.classify_offset(-30), Judgment::Perfect);
    assert_eq!(windows.classify_offset(31), Judgment::Great);
    assert_eq!(windows.classify_offset(-60), Judgment::Great);
    assert_eq!(windows.classify_offset(90), Judgment::Good);
    assert_eq!(windows.classify_offset(-90), Judgment::Good);
    assert_eq!(windows.classify_offset(91), Judgment::Miss);
}
