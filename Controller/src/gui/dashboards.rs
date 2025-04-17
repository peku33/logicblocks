use super::fontawesome::Icon;
use crate::{
    devices::Id as DeviceId,
    web::{self, uri_cursor},
};
use futures::future::{BoxFuture, FutureExt};
use itertools::Itertools;
use serde::Serialize;

#[derive(Debug)]
pub struct Dashboard {
    name: String,
    icon: Icon,

    content: Content,
}
impl uri_cursor::Handler for Dashboard {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        #[derive(Serialize)]
        struct DashboardSummarySerialize {
            name: String,
            icon: Icon,
        }
        impl DashboardSummarySerialize {
            pub fn new(dashboard: &Dashboard) -> Self {
                Self {
                    name: dashboard.name.clone(),
                    icon: dashboard.icon.clone(),
                }
            }
        }

        #[derive(Serialize)]
        struct DashboardContentSerialize {
            content: ContentSerialize,
        }
        impl DashboardContentSerialize {
            pub fn new(dashboard: &Dashboard) -> Self {
                Self {
                    content: ContentSerialize::new(&dashboard.content),
                }
            }
        }

        #[derive(Serialize)]
        #[serde(tag = "type")]
        enum ContentSerialize {
            SectionContent(ContentSectionContentSerialize),
            Sections(ContentSectionsSerialize),
        }
        impl ContentSerialize {
            fn new(content: &Content) -> Self {
                match content {
                    Content::SectionContent(content_section_content) => Self::SectionContent(
                        ContentSectionContentSerialize::new(content_section_content),
                    ),
                    Content::Sections(content_sections) => {
                        Self::Sections(ContentSectionsSerialize::new(content_sections))
                    }
                }
            }
        }

        #[derive(Serialize)]
        struct ContentSectionContentSerialize {
            section_content: SectionContentSerialize,
        }
        impl ContentSectionContentSerialize {
            fn new(content_section_content: &ContentSectionContent) -> Self {
                Self {
                    section_content: SectionContentSerialize::new(
                        &content_section_content.section_content,
                    ),
                }
            }
        }

        #[derive(Serialize)]
        struct ContentSectionsSerialize {
            sections: Box<[SectionSerialize]>,
        }
        impl ContentSectionsSerialize {
            fn new(content_sections: &ContentSections) -> Self {
                Self {
                    sections: content_sections
                        .sections
                        .iter()
                        .map(SectionSerialize::new)
                        .collect::<Box<[_]>>(),
                }
            }
        }

        #[derive(Serialize)]
        struct SectionSerialize {
            name: Option<String>,

            content: SectionContentSerialize,
        }
        impl SectionSerialize {
            fn new(section: &Section) -> Self {
                Self {
                    name: section.name.clone(),

                    content: SectionContentSerialize::new(&section.content),
                }
            }
        }

        #[derive(Serialize)]
        #[serde(tag = "type")]
        enum SectionContentSerialize {
            Dashboards(SectionContentDashboardsSerialize),
            Devices(SectionContentDevicesSerialize),
        }
        impl SectionContentSerialize {
            fn new(section_content: &SectionContent) -> Self {
                match section_content {
                    SectionContent::Dashboards(section_content_dashboards) => Self::Dashboards(
                        SectionContentDashboardsSerialize::new(section_content_dashboards),
                    ),
                    SectionContent::Devices(section_content_devices) => {
                        Self::Devices(SectionContentDevicesSerialize::new(section_content_devices))
                    }
                }
            }
        }

        #[derive(Serialize)]
        struct SectionContentDashboardsSerialize {
            dashboards: Box<[DashboardSummarySerialize]>,
        }
        impl SectionContentDashboardsSerialize {
            fn new(section_content_dashboards: &SectionContentDashboards) -> Self {
                Self {
                    dashboards: section_content_dashboards
                        .dashboards
                        .iter()
                        .map(DashboardSummarySerialize::new)
                        .collect::<Box<[_]>>(),
                }
            }
        }

        #[derive(Serialize)]
        struct SectionContentDevicesSerialize {
            device_ids: Box<[DeviceId]>,
        }
        impl SectionContentDevicesSerialize {
            fn new(section_content_devices: &SectionContentDevices) -> Self {
                Self {
                    device_ids: section_content_devices.device_ids.clone(),
                }
            }
        }

        match uri_cursor {
            uri_cursor::UriCursor::Next("summary", uri_cursor) => match uri_cursor.as_ref() {
                uri_cursor::UriCursor::Terminal => match *request.method() {
                    http::Method::GET => {
                        let dashboard_summary_serialize = DashboardSummarySerialize::new(self);

                        async { web::Response::ok_json(dashboard_summary_serialize) }.boxed()
                    }
                    _ => async { web::Response::error_405() }.boxed(),
                },
                _ => async { web::Response::error_404() }.boxed(),
            },
            uri_cursor::UriCursor::Next("content", uri_cursor) => match uri_cursor.as_ref() {
                uri_cursor::UriCursor::Terminal => match *request.method() {
                    http::Method::GET => {
                        let dashboard_content_serialize = DashboardContentSerialize::new(self);

                        async { web::Response::ok_json(dashboard_content_serialize) }.boxed()
                    }
                    _ => async { web::Response::error_405() }.boxed(),
                },
                _ => async { web::Response::error_404() }.boxed(),
            },
            uri_cursor::UriCursor::Next(content_path_item, uri_cursor) => {
                match &self.content {
                    // we expect dashboard_index
                    Content::SectionContent(section_content) => {
                        let dashboard_index: usize = match content_path_item.parse() {
                            Ok(dashboard_index) => dashboard_index,
                            Err(_) => return async { web::Response::error_404() }.boxed(),
                        };

                        let dashboards = match &section_content.section_content {
                            SectionContent::Dashboards(dashboards) => dashboards,
                            SectionContent::Devices(_) => {
                                return async { web::Response::error_404() }.boxed();
                            }
                        };
                        let dashboard = match dashboards.dashboards.get(dashboard_index) {
                            Some(dashboard) => dashboard,
                            None => return async { web::Response::error_404() }.boxed(),
                        };

                        dashboard.handle(request, uri_cursor)
                    }
                    Content::Sections(sections) => {
                        // we expect section_index:dashboard_index
                        let (section_index, dashboard_index) =
                            match content_path_item.split(':').collect_tuple() {
                                Some((section_index, dashboard_index)) => {
                                    (section_index, dashboard_index)
                                }
                                None => return async { web::Response::error_404() }.boxed(),
                            };

                        let section_index: usize = match section_index.parse() {
                            Ok(section_index) => section_index,
                            Err(_) => return async { web::Response::error_404() }.boxed(),
                        };
                        let dashboard_index: usize = match dashboard_index.parse() {
                            Ok(dashboard_index) => dashboard_index,
                            Err(_) => return async { web::Response::error_404() }.boxed(),
                        };

                        let section = match sections.sections.get(section_index) {
                            Some(section) => section,
                            None => return async { web::Response::error_404() }.boxed(),
                        };

                        let dashboards = match &section.content {
                            SectionContent::Dashboards(dashboards) => dashboards,
                            SectionContent::Devices(_) => {
                                return async { web::Response::error_404() }.boxed();
                            }
                        };
                        let dashboard = match dashboards.dashboards.get(dashboard_index) {
                            Some(dashboard) => dashboard,
                            None => return async { web::Response::error_404() }.boxed(),
                        };

                        dashboard.handle(request, uri_cursor)
                    }
                }
            }
            _ => async { web::Response::error_404() }.boxed(),
        }
    }
}

#[derive(Debug)]
pub enum Content {
    SectionContent(ContentSectionContent),
    Sections(ContentSections),
}

#[derive(Debug)]
pub struct ContentSectionContent {
    section_content: SectionContent,
}

#[derive(Debug)]
pub struct ContentSections {
    sections: Box<[Section]>,
}

#[derive(Debug)]
pub struct Section {
    name: Option<String>,

    content: SectionContent,
}

#[derive(Debug)]
pub enum SectionContent {
    Dashboards(SectionContentDashboards),
    Devices(SectionContentDevices),
}

#[derive(Debug)]
pub struct SectionContentDashboards {
    dashboards: Box<[Dashboard]>,
}

#[derive(Debug)]
pub struct SectionContentDevices {
    device_ids: Box<[DeviceId]>,
}

pub mod builder {
    use super::*;
    use crate::devices::helpers::DeviceHandleErased;

    pub trait IntoContent: Sized {
        fn into_dashboard(
            self,
            name: String,
            icon: Icon,
        ) -> Dashboard {
            Dashboard {
                name,
                icon,
                content: self.into_content(),
            }
        }

        fn into_content(self) -> Content;
    }

    pub trait IntoSectionContent: Sized {
        fn into_content(self) -> Content {
            Content::SectionContent(ContentSectionContent {
                section_content: self.into_section_content(),
            })
        }
        fn into_section(
            self,
            name: Option<String>,
        ) -> Section {
            Section {
                name,
                content: self.into_section_content(),
            }
        }

        fn into_section_content(self) -> SectionContent;
    }
    impl<T: IntoSectionContent> IntoContent for T {
        fn into_content(self) -> Content {
            self.into_content()
        }
    }

    pub struct ContentSectionsBuilder {
        sections: Vec<Section>,
    }
    impl ContentSectionsBuilder {
        pub fn new() -> Self {
            Self {
                sections: Vec::<Section>::new(),
            }
        }

        pub fn add(
            &mut self,
            section: Section,
        ) {
            self.sections.push(section);
        }
        pub fn add_with(
            mut self,
            section: Section,
        ) -> Self {
            self.add(section);
            self
        }

        pub fn into_content(self) -> Content {
            Content::Sections(self.into_content_sections())
        }
        pub fn into_content_sections(self) -> ContentSections {
            ContentSections {
                sections: self.sections.into_boxed_slice(),
            }
        }
    }
    impl IntoContent for ContentSectionsBuilder {
        fn into_content(self) -> Content {
            Content::Sections(self.into_content_sections())
        }
    }

    pub struct DashboardListBuilder {
        dashboards: Vec<Dashboard>,
    }
    impl DashboardListBuilder {
        pub fn new() -> Self {
            Self {
                dashboards: Vec::<Dashboard>::new(),
            }
        }

        pub fn add(
            &mut self,
            dashboard: Dashboard,
        ) {
            self.dashboards.push(dashboard);
        }
        pub fn add_with(
            mut self,
            dashboard: Dashboard,
        ) -> Self {
            self.add(dashboard);
            self
        }

        pub fn into_section_content_dashboards(self) -> SectionContentDashboards {
            SectionContentDashboards {
                dashboards: self.dashboards.into_boxed_slice(),
            }
        }
    }
    impl IntoSectionContent for DashboardListBuilder {
        fn into_section_content(self) -> SectionContent {
            SectionContent::Dashboards(self.into_section_content_dashboards())
        }
    }

    pub struct DeviceListBuilder {
        device_ids: Vec<DeviceId>,
    }
    impl DeviceListBuilder {
        pub fn new() -> Self {
            Self {
                device_ids: Vec::<DeviceId>::new(),
            }
        }

        pub fn add(
            &mut self,
            device_handle_erased: DeviceHandleErased,
        ) {
            self.device_ids.push(device_handle_erased.device_id());
        }
        pub fn add_with(
            mut self,
            device_handle_erased: DeviceHandleErased,
        ) -> Self {
            self.add(device_handle_erased);
            self
        }

        pub fn into_section_content(self) -> SectionContent {
            SectionContent::Devices(self.into_section_content_devices())
        }
        pub fn into_section_content_devices(self) -> SectionContentDevices {
            SectionContentDevices {
                device_ids: self.device_ids.into_boxed_slice(),
            }
        }
    }
    impl IntoSectionContent for DeviceListBuilder {
        fn into_section_content(self) -> SectionContent {
            SectionContent::Devices(self.into_section_content_devices())
        }
    }
}
