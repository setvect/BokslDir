#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Korean,
}

impl Default for Language {
    fn default() -> Self {
        Self::English
    }
}

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Korean => "ko",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "ko" => Language::Korean,
            _ => Language::English,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Korean => "한국어",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKey {
    Ok,
    Cancel,
    Hidden,
    LayoutDual,
    LayoutSingle,
    LayoutWarn,
    SortName,
    SortSize,
    SortDate,
    SortExt,
    FilterPrefix,
    MenuFile,
    MenuEdit,
    MenuView,
    MenuSettings,
    MenuHelp,
    MenuLanguage,
    WarnTitle,
    WarnCurrent,
    WarnRequired,
    WarnHint,
    PanelHeaderName,
    PanelHeaderNameExt,
    PanelHeaderSize,
    PanelHeaderModified,
    PanelHeaderCreated,
    PanelHeaderPermissions,
    PanelHeaderType,
    PanelHeaderOwner,
    DialogSuggestions,
    DialogSuggestionHint,
    DialogTitleFileExists,
    DialogSource,
    DialogUnknown,
    DialogTargetExists,
    DialogOverwrite,
    DialogSkip,
    DialogOverwriteAll,
    DialogSkipAll,
    DialogPressEscToCancel,
    DialogTitleDelete,
    DialogTrash,
    DialogDelete,
    DialogName,
    DialogPath,
    DialogType,
    DialogSize,
    DialogModified,
    DialogPermissions,
    DialogContents,
    DialogSearch,
    DialogSearchActive,
    DialogNoShortcutMatches,
    DialogHelpHint,
    DialogNewDirectory,
    DialogDirectoryName,
    DialogRename,
    DialogNewName,
    DialogBookmarkRename,
    DialogNewBookmarkName,
    DialogFilter,
    DialogFilterPattern,
    DialogKeyboardShortcutsTitle,
    AboutTitle,
    AboutBody,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKey {
    SizeFormatAutoToast,
    SizeFormatBytesToast,
    ProgressFilesCount,
    ProgressProcessed,
    DeleteHeader,
    DeleteMore,
    HelpTotal,
    HelpResults,
    MaxTabsPerPanel,
    TabCreated,
    TabClosed,
    CannotCloseLastTab,
    TabIndex,
    NoTabIndex,
    StatusLeftLong,
    StatusSelectedLong,
    LayoutDualToast,
    LayoutSingleToast,
}

#[derive(Debug, Clone, Copy)]
pub struct I18n {
    language: Language,
}

impl I18n {
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    pub fn language(self) -> Language {
        self.language
    }

    pub fn tr(self, key: TextKey) -> &'static str {
        match (self.language, key) {
            (Language::English, TextKey::Ok) => "OK",
            (Language::Korean, TextKey::Ok) => "확인",
            (Language::English, TextKey::Cancel) => "Cancel",
            (Language::Korean, TextKey::Cancel) => "취소",
            (Language::English, TextKey::Hidden) => "Hidden",
            (Language::Korean, TextKey::Hidden) => "숨김",
            (Language::English, TextKey::LayoutDual) => "DUAL",
            (Language::Korean, TextKey::LayoutDual) => "듀얼",
            (Language::English, TextKey::LayoutSingle) => "SINGLE",
            (Language::Korean, TextKey::LayoutSingle) => "싱글",
            (Language::English, TextKey::LayoutWarn) => "WARN",
            (Language::Korean, TextKey::LayoutWarn) => "경고",
            (Language::English, TextKey::SortName) => "Name",
            (Language::Korean, TextKey::SortName) => "이름",
            (Language::English, TextKey::SortSize) => "Size",
            (Language::Korean, TextKey::SortSize) => "크기",
            (Language::English, TextKey::SortDate) => "Date",
            (Language::Korean, TextKey::SortDate) => "날짜",
            (Language::English, TextKey::SortExt) => "Ext",
            (Language::Korean, TextKey::SortExt) => "확장자",
            (Language::English, TextKey::FilterPrefix) => "Filter",
            (Language::Korean, TextKey::FilterPrefix) => "필터",
            (Language::English, TextKey::MenuFile) => "File(F)",
            (Language::Korean, TextKey::MenuFile) => "파일(F)",
            (Language::English, TextKey::MenuEdit) => "Edit(E)",
            (Language::Korean, TextKey::MenuEdit) => "편집(E)",
            (Language::English, TextKey::MenuView) => "View(V)",
            (Language::Korean, TextKey::MenuView) => "보기(V)",
            (Language::English, TextKey::MenuSettings) => "Settings(S)",
            (Language::Korean, TextKey::MenuSettings) => "설정(S)",
            (Language::English, TextKey::MenuHelp) => "Help(H)",
            (Language::Korean, TextKey::MenuHelp) => "도움말(H)",
            (Language::English, TextKey::MenuLanguage) => "Language",
            (Language::Korean, TextKey::MenuLanguage) => "언어",
            (Language::English, TextKey::WarnTitle) => "Terminal Too Small",
            (Language::Korean, TextKey::WarnTitle) => "터미널 크기가 너무 작습니다",
            (Language::English, TextKey::WarnCurrent) => "Current:",
            (Language::Korean, TextKey::WarnCurrent) => "현재:",
            (Language::English, TextKey::WarnRequired) => "Required:",
            (Language::Korean, TextKey::WarnRequired) => "필요:",
            (Language::English, TextKey::WarnHint) => "Please resize your terminal",
            (Language::Korean, TextKey::WarnHint) => "터미널 크기를 늘려주세요",
            (Language::English, TextKey::PanelHeaderName) => "Name",
            (Language::Korean, TextKey::PanelHeaderName) => "이름",
            (Language::English, TextKey::PanelHeaderNameExt) => "Name(Ext)",
            (Language::Korean, TextKey::PanelHeaderNameExt) => "이름(확장자)",
            (Language::English, TextKey::PanelHeaderSize) => "Size",
            (Language::Korean, TextKey::PanelHeaderSize) => "크기",
            (Language::English, TextKey::PanelHeaderModified) => "Modified",
            (Language::Korean, TextKey::PanelHeaderModified) => "수정일",
            (Language::English, TextKey::PanelHeaderCreated) => "Created",
            (Language::Korean, TextKey::PanelHeaderCreated) => "생성일",
            (Language::English, TextKey::PanelHeaderPermissions) => "Permissions",
            (Language::Korean, TextKey::PanelHeaderPermissions) => "권한",
            (Language::English, TextKey::PanelHeaderType) => "Type",
            (Language::Korean, TextKey::PanelHeaderType) => "종류",
            (Language::English, TextKey::PanelHeaderOwner) => "Owner",
            (Language::Korean, TextKey::PanelHeaderOwner) => "소유",
            (Language::English, TextKey::DialogSuggestions) => "Suggestions",
            (Language::Korean, TextKey::DialogSuggestions) => "추천",
            (Language::English, TextKey::DialogSuggestionHint) => {
                "Tab:Apply suggestion  Shift+Tab/Up/Down:Select"
            }
            (Language::Korean, TextKey::DialogSuggestionHint) => {
                "Tab:추천 적용  Shift+Tab/Up/Down:선택"
            }
            (Language::English, TextKey::DialogTitleFileExists) => " File Exists ",
            (Language::Korean, TextKey::DialogTitleFileExists) => " 파일 충돌 ",
            (Language::English, TextKey::DialogSource) => "Source:",
            (Language::Korean, TextKey::DialogSource) => "원본:",
            (Language::English, TextKey::DialogUnknown) => "unknown",
            (Language::Korean, TextKey::DialogUnknown) => "알 수 없음",
            (Language::English, TextKey::DialogTargetExists) => "Target already exists:",
            (Language::Korean, TextKey::DialogTargetExists) => "대상 경로가 이미 존재합니다:",
            (Language::English, TextKey::DialogOverwrite) => "Overwrite",
            (Language::Korean, TextKey::DialogOverwrite) => "덮어쓰기",
            (Language::English, TextKey::DialogSkip) => "Skip",
            (Language::Korean, TextKey::DialogSkip) => "건너뛰기",
            (Language::English, TextKey::DialogOverwriteAll) => "Overwrite All",
            (Language::Korean, TextKey::DialogOverwriteAll) => "모두 덮어쓰기",
            (Language::English, TextKey::DialogSkipAll) => "Skip All",
            (Language::Korean, TextKey::DialogSkipAll) => "모두 건너뛰기",
            (Language::English, TextKey::DialogPressEscToCancel) => "Press Esc to cancel",
            (Language::Korean, TextKey::DialogPressEscToCancel) => "Esc로 취소",
            (Language::English, TextKey::DialogTitleDelete) => " Delete ",
            (Language::Korean, TextKey::DialogTitleDelete) => " 삭제 ",
            (Language::English, TextKey::DialogTrash) => "Trash",
            (Language::Korean, TextKey::DialogTrash) => "휴지통",
            (Language::English, TextKey::DialogDelete) => "Delete",
            (Language::Korean, TextKey::DialogDelete) => "삭제",
            (Language::English, TextKey::DialogName) => "Name:",
            (Language::Korean, TextKey::DialogName) => "이름:",
            (Language::English, TextKey::DialogPath) => "Path:",
            (Language::Korean, TextKey::DialogPath) => "경로:",
            (Language::English, TextKey::DialogType) => "Type:",
            (Language::Korean, TextKey::DialogType) => "종류:",
            (Language::English, TextKey::DialogSize) => "Size:",
            (Language::Korean, TextKey::DialogSize) => "크기:",
            (Language::English, TextKey::DialogModified) => "Modified:",
            (Language::Korean, TextKey::DialogModified) => "수정일:",
            (Language::English, TextKey::DialogPermissions) => "Permissions:",
            (Language::Korean, TextKey::DialogPermissions) => "권한:",
            (Language::English, TextKey::DialogContents) => "Contents:",
            (Language::Korean, TextKey::DialogContents) => "내용:",
            (Language::English, TextKey::DialogSearch) => "Search:",
            (Language::Korean, TextKey::DialogSearch) => "검색:",
            (Language::English, TextKey::DialogSearchActive) => "Search*:",
            (Language::Korean, TextKey::DialogSearchActive) => "검색*:",
            (Language::English, TextKey::DialogNoShortcutMatches) => {
                "No shortcuts match your search"
            }
            (Language::Korean, TextKey::DialogNoShortcutMatches) => "검색 결과가 없습니다",
            (Language::English, TextKey::DialogHelpHint) => "Esc:Clear/Close  /:Search  j/k:Scroll",
            (Language::Korean, TextKey::DialogHelpHint) => "Esc:닫기  /:검색  j/k:스크롤",
            (Language::English, TextKey::DialogNewDirectory) => "New Directory",
            (Language::Korean, TextKey::DialogNewDirectory) => "새 폴더",
            (Language::English, TextKey::DialogDirectoryName) => "Directory name:",
            (Language::Korean, TextKey::DialogDirectoryName) => "폴더 이름:",
            (Language::English, TextKey::DialogRename) => "Rename",
            (Language::Korean, TextKey::DialogRename) => "이름 변경",
            (Language::English, TextKey::DialogNewName) => "New name:",
            (Language::Korean, TextKey::DialogNewName) => "새 이름:",
            (Language::English, TextKey::DialogBookmarkRename) => "Bookmark Rename",
            (Language::Korean, TextKey::DialogBookmarkRename) => "북마크 이름 변경",
            (Language::English, TextKey::DialogNewBookmarkName) => "New bookmark name:",
            (Language::Korean, TextKey::DialogNewBookmarkName) => "새 북마크 이름:",
            (Language::English, TextKey::DialogFilter) => "Filter",
            (Language::Korean, TextKey::DialogFilter) => "필터",
            (Language::English, TextKey::DialogFilterPattern) => "Pattern (supports * ?):",
            (Language::Korean, TextKey::DialogFilterPattern) => "패턴 (* ? 지원):",
            (Language::English, TextKey::DialogKeyboardShortcutsTitle) => " Keyboard Shortcuts ",
            (Language::Korean, TextKey::DialogKeyboardShortcutsTitle) => " 단축키 도움말 ",
            (Language::English, TextKey::AboutTitle) => "About BokslDir",
            (Language::Korean, TextKey::AboutTitle) => "복슬Dir 정보",
            (Language::English, TextKey::AboutBody) => "BokslDir\nRust TUI dual-panel file manager",
            (Language::Korean, TextKey::AboutBody) => {
                "복슬Dir\nRust 기반 TUI 듀얼 패널 파일 매니저"
            }
        }
    }

    pub fn msg(self, key: MessageKey) -> &'static str {
        match (self.language, key) {
            (Language::English, MessageKey::SizeFormatAutoToast) => "Size format: Auto",
            (Language::Korean, MessageKey::SizeFormatAutoToast) => "크기 표시: 자동",
            (Language::English, MessageKey::SizeFormatBytesToast) => "Size format: Bytes",
            (Language::Korean, MessageKey::SizeFormatBytesToast) => "크기 표시: 바이트",
            (Language::English, MessageKey::ProgressFilesCount) => "{completed} / {total} files",
            (Language::Korean, MessageKey::ProgressFilesCount) => "{completed} / {total} 파일",
            (Language::English, MessageKey::ProgressProcessed) => {
                "Processed: {processed}  Remaining: {remaining}  Failed: {failed}"
            }
            (Language::Korean, MessageKey::ProgressProcessed) => {
                "처리: {processed}  남음: {remaining}  실패: {failed}"
            }
            (Language::English, MessageKey::DeleteHeader) => "Delete {count} items? ({total_size})",
            (Language::Korean, MessageKey::DeleteHeader) => {
                "{count}개 항목을 삭제할까요? ({total_size})"
            }
            (Language::English, MessageKey::DeleteMore) => "  ... and {count} more",
            (Language::Korean, MessageKey::DeleteMore) => "  ... 외 {count}개",
            (Language::English, MessageKey::HelpTotal) => "Total: {count}",
            (Language::Korean, MessageKey::HelpTotal) => "전체: {count}",
            (Language::English, MessageKey::HelpResults) => "Results: {count}",
            (Language::Korean, MessageKey::HelpResults) => "결과: {count}",
            (Language::English, MessageKey::MaxTabsPerPanel) => "Max 5 tabs per panel",
            (Language::Korean, MessageKey::MaxTabsPerPanel) => "패널당 탭은 최대 5개입니다",
            (Language::English, MessageKey::TabCreated) => "Tab created ({index})",
            (Language::Korean, MessageKey::TabCreated) => "탭 생성 ({index})",
            (Language::English, MessageKey::TabClosed) => "Tab closed",
            (Language::Korean, MessageKey::TabClosed) => "탭을 닫았습니다",
            (Language::English, MessageKey::CannotCloseLastTab) => "Cannot close last tab",
            (Language::Korean, MessageKey::CannotCloseLastTab) => "마지막 탭은 닫을 수 없습니다",
            (Language::English, MessageKey::TabIndex) => "Tab {index}",
            (Language::Korean, MessageKey::TabIndex) => "탭 {index}",
            (Language::English, MessageKey::NoTabIndex) => "No tab {index}",
            (Language::Korean, MessageKey::NoTabIndex) => "탭 없음 {index}",
            (Language::English, MessageKey::StatusLeftLong) => {
                " {files} files, {dirs} dirs | {total}"
            }
            (Language::Korean, MessageKey::StatusLeftLong) => {
                " 파일 {files}개, 폴더 {dirs}개 | {total}"
            }
            (Language::English, MessageKey::StatusSelectedLong) => " | {count} selected ({size})",
            (Language::Korean, MessageKey::StatusSelectedLong) => " | 선택 {count}개 ({size})",
            (Language::English, MessageKey::LayoutDualToast) => "Layout: Dual panel",
            (Language::Korean, MessageKey::LayoutDualToast) => "레이아웃: 듀얼 패널",
            (Language::English, MessageKey::LayoutSingleToast) => "Layout: Single panel",
            (Language::Korean, MessageKey::LayoutSingleToast) => "레이아웃: 싱글 패널",
        }
    }

    pub fn fmt(self, key: MessageKey, args: &[(&str, String)]) -> String {
        let mut out = self.msg(key).to_string();
        for (name, value) in args {
            let needle = format!("{{{}}}", name);
            out = out.replace(&needle, value);
        }
        out
    }

    pub fn sort_indicator(self, sort_name_key: TextKey, ascending: bool) -> String {
        let arrow = if ascending { "▲" } else { "▼" };
        format!("{} {}", self.tr(sort_name_key), arrow)
    }

    pub fn filter_indicator(self, pattern: &str) -> String {
        format!("{}: {}", self.tr(TextKey::FilterPrefix), pattern)
    }

    pub fn menu_item(self, id: &str) -> &'static str {
        match (self.language, id) {
            (Language::English, "new_dir") => "New Directory",
            (Language::Korean, "new_dir") => "새 폴더",
            (Language::English, "open_default") => "Open with default app",
            (Language::Korean, "open_default") => "기본 프로그램으로 열기",
            (Language::English, "open_terminal_editor") => "Open in terminal editor",
            (Language::Korean, "open_terminal_editor") => "터미널 에디터로 열기",
            (Language::English, "archive_compress") => "Compress",
            (Language::Korean, "archive_compress") => "압축",
            (Language::English, "archive_extract") => "Extract",
            (Language::Korean, "archive_extract") => "압축 해제",
            (Language::English, "archive_extract_auto") => "Auto extract",
            (Language::Korean, "archive_extract_auto") => "알아서 풀기",
            (Language::English, "archive_preview") => "Archive preview",
            (Language::Korean, "archive_preview") => "압축 미리보기",
            (Language::English, "rename") => "Rename",
            (Language::Korean, "rename") => "이름 변경",
            (Language::English, "delete") => "Delete",
            (Language::Korean, "delete") => "삭제",
            (Language::English, "perm_delete") => "Permanent delete",
            (Language::Korean, "perm_delete") => "영구 삭제",
            (Language::English, "quit") => "Quit",
            (Language::Korean, "quit") => "종료",
            (Language::English, "copy") => "Copy",
            (Language::Korean, "copy") => "복사",
            (Language::English, "move") => "Move",
            (Language::Korean, "move") => "이동",
            (Language::English, "select_all") => "Select all",
            (Language::Korean, "select_all") => "전체 선택",
            (Language::English, "invert_selection") => "Invert selection",
            (Language::Korean, "invert_selection") => "선택 반전",
            (Language::English, "deselect") => "Deselect all",
            (Language::Korean, "deselect") => "선택 해제",
            (Language::English, "refresh") => "Refresh",
            (Language::Korean, "refresh") => "새로고침",
            (Language::English, "file_info") => "File info",
            (Language::Korean, "file_info") => "파일 정보",
            (Language::English, "sort_name") => "Name",
            (Language::Korean, "sort_name") => "이름",
            (Language::English, "sort_size") => "Size",
            (Language::Korean, "sort_size") => "크기",
            (Language::English, "sort_date") => "Modified date",
            (Language::Korean, "sort_date") => "수정 날짜",
            (Language::English, "sort_ext") => "Extension",
            (Language::Korean, "sort_ext") => "확장자",
            (Language::English, "sort_asc") => "Ascending",
            (Language::Korean, "sort_asc") => "오름차순",
            (Language::English, "sort_desc") => "Descending",
            (Language::Korean, "sort_desc") => "내림차순",
            (Language::English, "filter_start") => "Filter",
            (Language::Korean, "filter_start") => "필터링",
            (Language::English, "filter_clear") => "Clear filter",
            (Language::Korean, "filter_clear") => "필터 해제",
            (Language::English, "toggle_hidden") => "Show hidden files",
            (Language::Korean, "toggle_hidden") => "숨김 파일 표시",
            (Language::English, "toggle_layout") => "Toggle single/dual panel",
            (Language::Korean, "toggle_layout") => "싱글/듀얼 패널 전환",
            (Language::English, "mount_points") => "Mount points",
            (Language::Korean, "mount_points") => "마운트 포인트",
            (Language::English, "goto_path") => "Go to path",
            (Language::Korean, "goto_path") => "경로로 이동",
            (Language::English, "history_list") => "Directory history",
            (Language::Korean, "history_list") => "디렉토리 히스토리",
            (Language::English, "bookmark_list") => "Bookmarks",
            (Language::Korean, "bookmark_list") => "북마크",
            (Language::English, "size_auto") => "Auto (KB/MB/GB)",
            (Language::Korean, "size_auto") => "자동 (KB/MB/GB)",
            (Language::English, "size_bytes") => "Bytes",
            (Language::Korean, "size_bytes") => "바이트",
            (Language::English, "toggle_icons") => "Toggle icons",
            (Language::Korean, "toggle_icons") => "아이콘 전환",
            (Language::English, "help_keys") => "Keyboard help",
            (Language::Korean, "help_keys") => "단축키 도움말",
            (Language::English, "about") => "About BokslDir",
            (Language::Korean, "about") => "복슬Dir 정보",
            _ => "",
        }
    }

    pub fn menu_group(self, id: &str) -> &'static str {
        match (self.language, id) {
            (Language::English, "sort_by") => "Sort by",
            (Language::Korean, "sort_by") => "정렬 기준",
            (Language::English, "sort_order") => "Sort order",
            (Language::Korean, "sort_order") => "정렬 순서",
            (Language::English, "size_format") => "Size format",
            (Language::Korean, "size_format") => "크기 표시 형식",
            (Language::English, "theme") => "Theme",
            (Language::Korean, "theme") => "테마",
            (Language::English, "default_editor") => "Default editor",
            (Language::Korean, "default_editor") => "기본 에디터",
            _ => "",
        }
    }

    pub fn help_category(self, id: &str) -> &'static str {
        match (self.language, id) {
            (Language::English, "navigation") => "Navigation",
            (Language::Korean, "navigation") => "탐색",
            (Language::English, "file_operation") => "File Operations",
            (Language::Korean, "file_operation") => "파일 작업",
            (Language::English, "selection") => "Selection",
            (Language::Korean, "selection") => "선택",
            (Language::English, "sort") => "Sort",
            (Language::Korean, "sort") => "정렬",
            (Language::English, "filter") => "Filter / Search",
            (Language::Korean, "filter") => "필터 / 검색",
            (Language::English, "system") => "System",
            (Language::Korean, "system") => "시스템",
            _ => "",
        }
    }

    pub fn action_help_label(self, id: &str, fallback: &'static str) -> &'static str {
        match (self.language, id) {
            (Language::English, _) => fallback,
            (Language::Korean, "move_up") => "위/아래 이동",
            (Language::Korean, "move_down") => "아래로 이동",
            (Language::Korean, "go_parent") => "상위 폴더",
            (Language::Korean, "enter") => "선택 항목 열기",
            (Language::Korean, "go_top") => "맨 위/아래",
            (Language::Korean, "go_bottom") => "맨 아래",
            (Language::Korean, "page_up") => "페이지 이동",
            (Language::Korean, "page_down") => "아래 페이지 이동",
            (Language::Korean, "toggle_panel") => "패널 전환",
            (Language::Korean, "toggle_layout") => "레이아웃 전환",
            (Language::Korean, "tab_new") => "새 탭",
            (Language::Korean, "tab_close") => "탭 닫기",
            (Language::Korean, "copy") => "복사",
            (Language::Korean, "move") => "이동",
            (Language::Korean, "open_default") => "기본 프로그램으로 열기",
            (Language::Korean, "open_terminal_editor") => "터미널 에디터로 열기",
            (Language::Korean, "delete") => "삭제",
            (Language::Korean, "perm_delete") => "영구삭제",
            (Language::Korean, "new_dir") => "새폴더",
            (Language::Korean, "rename") => "이름변경",
            (Language::Korean, "file_info") => "정보",
            (Language::Korean, "archive_compress") => "압축",
            (Language::Korean, "archive_extract") => "압축 해제",
            (Language::Korean, "archive_extract_auto") => "알아서 풀기",
            (Language::Korean, "archive_preview") => "압축 미리보기",
            (Language::Korean, "toggle_sel") => "선택 토글",
            (Language::Korean, "toggle_select") => "선택 토글",
            (Language::Korean, "invert_selection") => "선택 반전",
            (Language::Korean, "select_all") => "전체 선택",
            (Language::Korean, "deselect") => "전체 해제",
            (Language::Korean, "help_keys") => "단축키",
            (Language::Korean, "refresh") => "새로고침",
            (Language::Korean, "open_menu") => "메뉴 열기",
            (Language::Korean, "quit") => "종료",
            (Language::Korean, "theme_dark") => "다크 테마",
            (Language::Korean, "theme_light") => "라이트 테마",
            (Language::Korean, "theme_contrast") => "고대비 테마",
            (Language::Korean, "language_en") => "언어: 영어",
            (Language::Korean, "language_ko") => "언어: 한국어",
            (Language::Korean, "toggle_icons") => "아이콘 전환",
            (Language::Korean, "editor_preset_vi") => "기본 에디터: vi",
            (Language::Korean, "editor_preset_vim") => "기본 에디터: vim",
            (Language::Korean, "editor_preset_nano") => "기본 에디터: nano",
            (Language::Korean, "editor_preset_emacs") => "기본 에디터: emacs",
            (Language::Korean, "sort_name") => "이름순 정렬",
            (Language::Korean, "sort_size") => "크기순 정렬",
            (Language::Korean, "sort_date") => "날짜순 정렬",
            (Language::Korean, "sort_ext") => "확장자순 정렬",
            (Language::Korean, "sort_asc") => "정렬 순서 반전",
            (Language::Korean, "sort_desc") => "내림차순",
            (Language::Korean, "filter_start") => "필터",
            (Language::Korean, "filter_clear") => "필터 해제",
            (Language::Korean, "toggle_hidden") => "숨김 파일 표시 전환",
            (Language::Korean, "mount_points") => "마운트 포인트",
            (Language::Korean, "goto_path") => "경로로 이동",
            (Language::Korean, "tab_list") => "탭 목록 보기",
            (Language::Korean, "history_back") => "히스토리 뒤로",
            (Language::Korean, "history_forward") => "히스토리 앞으로",
            (Language::Korean, "history_list") => "히스토리 목록 보기",
            (Language::Korean, "bookmark_add") => "북마크 추가",
            (Language::Korean, "bookmark_list") => "북마크 목록 보기",
            (Language::Korean, "size_auto") => "크기: 자동",
            (Language::Korean, "size_bytes") => "크기: 바이트",
            (Language::Korean, "about") => "정보",
            _ => fallback,
        }
    }
}

pub fn localize_runtime_text(language: Language, input: &str) -> String {
    if matches!(language, Language::English) {
        return input.to_string();
    }

    let exact = match input {
        "Error" => Some("오류"),
        "Information" => Some("정보"),
        "Mount Points" => Some("마운트 포인트"),
        "History" => Some("히스토리"),
        "Bookmarks" => Some("북마크"),
        "Bookmark deleted" => Some("북마크를 삭제했습니다"),
        "Bookmark renamed" => Some("북마크 이름을 변경했습니다"),
        "Bookmark name cannot be empty" => Some("북마크 이름은 비울 수 없습니다"),
        "Bookmark already exists" => Some("이미 존재하는 북마크입니다"),
        "Failed to open bookmark path" => Some("북마크 경로를 열지 못했습니다"),
        "Failed to open history path" => Some("히스토리 경로를 열지 못했습니다"),
        "History cleared" => Some("히스토리를 비웠습니다"),
        "History back failed" => Some("뒤로 이동에 실패했습니다"),
        "No back history" => Some("뒤로 갈 히스토리가 없습니다"),
        "History forward failed" => Some("앞으로 이동에 실패했습니다"),
        "No forward history" => Some("앞으로 갈 히스토리가 없습니다"),
        "Filter cleared" => Some("필터를 해제했습니다"),
        "Hidden files shown" => Some("숨김 파일 표시"),
        "Hidden files hidden" => Some("숨김 파일 숨김"),
        "Rename completed" => Some("이름 변경 완료"),
        "Password is empty." => Some("비밀번호가 비어 있습니다."),
        "Password and confirmation do not match." => Some("비밀번호와 확인 값이 다릅니다."),
        _ => None,
    };
    if let Some(msg) = exact {
        return msg.to_string();
    }

    if let Some(value) = input.strip_prefix("Default editor: ") {
        return format!("기본 에디터: {}", value);
    }
    if let Some(value) = input.strip_prefix("Opened: ") {
        return format!("열기 완료: {}", value);
    }
    if let Some(value) = input.strip_prefix("Edited: ") {
        return format!("편집 완료: {}", value);
    }
    if let Some(value) = input.strip_prefix("Bookmark added: ") {
        return format!("북마크 추가: {}", value);
    }
    if let Some(value) = input.strip_prefix("Filter: ") {
        return format!("필터: {}", value);
    }
    if let Some(value) = input.strip_prefix("Archive already exists: ") {
        return format!("압축 파일이 이미 존재합니다: {}", value);
    }
    if input.starts_with("Size format: Auto") {
        return I18n::new(language)
            .msg(MessageKey::SizeFormatAutoToast)
            .to_string();
    }
    if input.starts_with("Size format: Bytes") {
        return I18n::new(language)
            .msg(MessageKey::SizeFormatBytesToast)
            .to_string();
    }
    input.to_string()
}
