// IME (Input Method Editor) status detection
//
// macOS: Carbon TIS API를 통해 현재 입력 소스를 감지
// Linux: 현재 미지원 (None 반환)

/// IME 상태
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeStatus {
    /// 한글 입력 모드
    Korean,
    /// 영문 입력 모드
    English,
    /// 기타 입력 소스
    Other(String),
    /// 감지 불가
    Unknown,
}

impl ImeStatus {
    /// 상태바 표시용 문자열
    pub fn display_label(&self) -> &str {
        match self {
            ImeStatus::Korean => "한글",
            ImeStatus::English => "EN",
            ImeStatus::Other(name) => name.as_str(),
            ImeStatus::Unknown => "",
        }
    }

    /// 상태바에 표시할 필요가 있는지
    pub fn should_display(&self) -> bool {
        !matches!(self, ImeStatus::Unknown)
    }
}

/// 현재 IME 상태를 조회
pub fn get_current_ime() -> ImeStatus {
    #[cfg(target_os = "macos")]
    {
        macos::get_ime_status()
    }
    #[cfg(not(target_os = "macos"))]
    {
        ImeStatus::Unknown
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::ImeStatus;
    use std::ffi::c_void;

    // Core Foundation types
    type CFStringRef = *const c_void;
    type CFTypeRef = *const c_void;
    type CFIndex = isize;
    type CFStringEncoding = u32;
    type CFTimeInterval = f64;
    type CFRunLoopRef = *const c_void;
    type CFRunLoopMode = CFStringRef;

    // TIS types
    type TISInputSourceRef = *const c_void;

    // kCFStringEncodingUTF8 = 0x08000100
    const K_CF_STRING_ENCODING_UTF8: CFStringEncoding = 0x0800_0100;

    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
        fn TISGetInputSourceProperty(source: TISInputSourceRef, key: CFStringRef) -> CFTypeRef;
        static kTISPropertyInputSourceID: CFStringRef;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(cf: CFTypeRef);
        fn CFStringGetLength(s: CFStringRef) -> CFIndex;
        fn CFStringGetMaximumSizeForEncoding(
            length: CFIndex,
            encoding: CFStringEncoding,
        ) -> CFIndex;
        fn CFStringGetCString(
            s: CFStringRef,
            buffer: *mut u8,
            buffer_size: CFIndex,
            encoding: CFStringEncoding,
        ) -> bool;
        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CFRunLoopRunInMode(
            mode: CFRunLoopMode,
            seconds: CFTimeInterval,
            return_after_source_handled: bool,
        ) -> i32;
        static kCFRunLoopDefaultMode: CFRunLoopMode;
    }

    /// CFStringRef → Rust String 변환
    unsafe fn cfstring_to_string(cf_str: CFStringRef) -> Option<String> {
        if cf_str.is_null() {
            return None;
        }
        let len = unsafe { CFStringGetLength(cf_str) };
        if len <= 0 {
            return None;
        }
        let max_size =
            unsafe { CFStringGetMaximumSizeForEncoding(len, K_CF_STRING_ENCODING_UTF8) } + 1;
        let mut buffer = vec![0u8; max_size as usize];
        let ok = unsafe {
            CFStringGetCString(
                cf_str,
                buffer.as_mut_ptr(),
                max_size,
                K_CF_STRING_ENCODING_UTF8,
            )
        };
        if ok {
            // Find null terminator
            let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
            String::from_utf8(buffer[..end].to_vec()).ok()
        } else {
            None
        }
    }

    /// 입력 소스 ID로부터 IME 상태 판별
    fn classify_input_source(source_id: &str) -> ImeStatus {
        if source_id.contains("Korean") {
            ImeStatus::Korean
        } else if source_id.contains("keylayout.ABC")
            || source_id.contains("keylayout.US")
            || source_id.contains("keylayout.USInternational")
            || source_id.contains("keylayout.Colemak")
            || source_id.contains("keylayout.Dvorak")
            || source_id.contains("keylayout.QWERTZ")
            || source_id.contains("keylayout.British")
        {
            ImeStatus::English
        } else if source_id.starts_with("com.apple.keylayout.") {
            // 기타 키보드 레이아웃 → 영문 계열로 추정
            ImeStatus::English
        } else if source_id.contains("Japanese") {
            ImeStatus::Other("JP".to_string())
        } else if source_id.contains("Chinese") || source_id.contains("Pinyin") {
            ImeStatus::Other("CN".to_string())
        } else {
            ImeStatus::Other("IME".to_string())
        }
    }

    pub fn get_ime_status() -> ImeStatus {
        unsafe {
            // CFRunLoop을 짧게 펌핑하여 입력 소스 변경 알림을 처리
            // TISCopyCurrentKeyboardInputSource()는 CFRunLoop 알림을 통해
            // 입력 소스 변경을 감지하므로, 펌핑 없이는 캐시된 값만 반환됨
            let _ = CFRunLoopGetCurrent();
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.001, false);

            let source = TISCopyCurrentKeyboardInputSource();
            if source.is_null() {
                return ImeStatus::Unknown;
            }

            let source_id_ref = TISGetInputSourceProperty(source, kTISPropertyInputSourceID);
            let result = if source_id_ref.is_null() {
                ImeStatus::Unknown
            } else {
                // TISGetInputSourceProperty returns a borrowed CFStringRef (no release needed)
                match cfstring_to_string(source_id_ref as CFStringRef) {
                    Some(id) => classify_input_source(&id),
                    None => ImeStatus::Unknown,
                }
            };

            // TISCopyCurrentKeyboardInputSource returns a retained object → must release
            CFRelease(source);

            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ime_status_display() {
        assert_eq!(ImeStatus::Korean.display_label(), "한글");
        assert_eq!(ImeStatus::English.display_label(), "EN");
        assert_eq!(ImeStatus::Other("JP".to_string()).display_label(), "JP");
        assert_eq!(ImeStatus::Unknown.display_label(), "");
    }

    #[test]
    fn test_ime_status_should_display() {
        assert!(ImeStatus::Korean.should_display());
        assert!(ImeStatus::English.should_display());
        assert!(!ImeStatus::Unknown.should_display());
    }

    #[test]
    fn test_get_current_ime_does_not_panic() {
        // 실제 OS에서 패닉 없이 호출되는지 확인
        let _status = get_current_ime();
    }
}
