use crate::models::Response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum Pane {
    Sidebar,
    WorkspaceTabs,
    #[default]
    UrlBar,
    MethodSelector,
    RequestTabs,
    RequestEditor,
    ResponseTabs,
    ResponseViewer,
}

impl Pane {
    pub fn next(&self) -> Self {
        match self {
            Pane::Sidebar => Pane::WorkspaceTabs,
            Pane::WorkspaceTabs => Pane::UrlBar,
            Pane::UrlBar => Pane::RequestTabs,
            Pane::RequestTabs => Pane::RequestEditor,
            Pane::RequestEditor => Pane::ResponseViewer,
            Pane::ResponseViewer => Pane::Sidebar,
            Pane::MethodSelector => Pane::UrlBar,
            Pane::ResponseTabs => Pane::ResponseViewer,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Pane::Sidebar => Pane::ResponseViewer,
            Pane::WorkspaceTabs => Pane::Sidebar,
            Pane::UrlBar => Pane::WorkspaceTabs,
            Pane::RequestTabs => Pane::UrlBar,
            Pane::RequestEditor => Pane::RequestTabs,
            Pane::ResponseViewer => Pane::RequestEditor,
            Pane::MethodSelector => Pane::UrlBar,
            Pane::ResponseTabs => Pane::RequestEditor,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SidebarViewMode {
    #[default]
    Collections,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RequestSubtab {
    #[default]
    Query,
    Headers,
    Auth,
    Body,
    Path,
}

impl RequestSubtab {
    pub const ALL: [RequestSubtab; 5] = [
        RequestSubtab::Query,
        RequestSubtab::Headers,
        RequestSubtab::Auth,
        RequestSubtab::Body,
        RequestSubtab::Path,
    ];

    pub fn index(&self) -> usize {
        match self {
            RequestSubtab::Query => 0,
            RequestSubtab::Headers => 1,
            RequestSubtab::Auth => 2,
            RequestSubtab::Body => 3,
            RequestSubtab::Path => 4,
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx % 5 {
            0 => RequestSubtab::Query,
            1 => RequestSubtab::Headers,
            2 => RequestSubtab::Auth,
            3 => RequestSubtab::Body,
            _ => RequestSubtab::Path,
        }
    }

    pub fn next(&self) -> Self {
        Self::from_index(self.index() + 1)
    }

    pub fn prev(&self) -> Self {
        Self::from_index(self.index() + 4)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ResponseSubtab {
    #[default]
    Body,
    Headers,
}

impl ResponseSubtab {
    pub const ALL: [ResponseSubtab; 2] = [ResponseSubtab::Body, ResponseSubtab::Headers];

    pub fn index(&self) -> usize {
        match self {
            ResponseSubtab::Body => 0,
            ResponseSubtab::Headers => 1,
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx % 2 {
            0 => ResponseSubtab::Body,
            _ => ResponseSubtab::Headers,
        }
    }

    pub fn next(&self) -> Self {
        Self::from_index(self.index() + 1)
    }

    pub fn prev(&self) -> Self {
        Self::from_index(self.index() + 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragTarget {
    SidebarWidth,
    RequestResponseSplit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Action {
    Quit,
    Tick,
    Render,
    FocusNext,
    FocusPrev,
    SwitchFocus(Pane),
    NextOpenTab,
    PrevOpenTab,
    SwitchOpenTab(usize),
    CloseOpenTab(usize),
    CloseCurrentTab,
    NewTab,
    NextRequestSubtab,
    PrevRequestSubtab,
    NextResponseSubtab,
    PrevResponseSubtab,
    NextMethod,
    PrevMethod,
    SendRequest,
    RequestStarted,
    ResponseReceived(usize, Response),
    RequestFailed(usize, String),
    InputChar(char),
    Backspace,
    Delete,
    CursorLeft,
    CursorRight,
    SubmitInput,
    ToggleRow,
    SelectNextRow,
    SelectPrevRow,
    AddTableRow,
    EditTableRow,
    DeleteTableRow,
    SelectNextCollection,
    SelectPrevCollection,
    LoadSelectedCollection,
    ToggleSidebarView,
    LoadSelectedHistory,
    ClearHistory,
    ScrollResponseUp,
    ScrollResponseDown,
    OpenSearchModal,
    NextSearchMatch,
    PrevSearchMatch,
    ClearSearch,
    OpenCommandPalette,
    SaveRequest,
    NextEnvironment,
    NextAuthType,
    PrevAuthType,
    CopyAsCurl,
    CopyResponseBody,
    OpenImportCurlModal,
    OpenImportModal,
    OpenExportModal,
    OpenNewRequestModal,
    OpenNewGroupModal,
    OpenDeleteRequestModal,
    OpenDeleteGroupModal,
    OpenClearHistoryModal,
    DeleteSelectedHistory,
    ScrollTabsLeft,
    ScrollTabsRight,
    ScrollSidebarUp,
    ScrollSidebarDown,
    CloseModal,
    SubmitModal,
    ModalNextField,
    ModalPrevField,
    ResizeSidebar(i16),
    ResizeVerticalSplit(i16),
    StartDrag(DragTarget),
    EndDrag,
}
