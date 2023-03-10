use super::fontawesome::Icon;
use crate::{
    devices::Id as DeviceId,
    web::{self, uri_cursor},
};
use futures::future::{BoxFuture, FutureExt};
use serde::Serialize;

#[derive(Debug)]
pub struct Dashboard {
    name: String,
    icon: Icon,

    sections: Vec<Section>,
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
            fn new(dashboard: &Dashboard) -> Self {
                Self {
                    name: dashboard.name.clone(),
                    icon: dashboard.icon.clone(),
                }
            }
        }

        match uri_cursor {
            uri_cursor::UriCursor::Next("summary", uri_cursor) => match uri_cursor.as_ref() {
                uri_cursor::UriCursor::Terminal => match *request.method() {
                    http::Method::GET => {
                        let dashboard_summary_serialize = DashboardSummarySerialize::new(self);

                        async move { web::Response::ok_json(dashboard_summary_serialize) }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                _ => async move { web::Response::error_404() }.boxed(),
            },
            uri_cursor::UriCursor::Next("content", uri_cursor) => match uri_cursor.as_ref() {
                uri_cursor::UriCursor::Terminal => match *request.method() {
                    http::Method::GET => {
                        #[derive(Serialize)]
                        #[serde(tag = "type")]
                        enum SectionContentSerialize {
                            Dashboards {
                                dashboards: Vec<DashboardSummarySerialize>,
                            },
                            Devices {
                                device_ids: Vec<DeviceId>,
                            },
                        }
                        impl SectionContentSerialize {
                            fn new(section_content: &SectionContent) -> Self {
                                match section_content {
                                    SectionContent::Dashboards(section_content_dashboards) => {
                                        Self::Dashboards {
                                            dashboards: section_content_dashboards
                                                .dashboards
                                                .iter()
                                                .map(DashboardSummarySerialize::new)
                                                .collect::<Vec<_>>(),
                                        }
                                    }
                                    SectionContent::Devices(section_content_devices) => {
                                        Self::Devices {
                                            device_ids: section_content_devices.device_ids.clone(),
                                        }
                                    }
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
                        struct DashboardContentSerialize {
                            sections: Vec<SectionSerialize>,
                        }
                        impl DashboardContentSerialize {
                            fn new(dashboard: &Dashboard) -> Self {
                                Self {
                                    sections: dashboard
                                        .sections
                                        .iter()
                                        .map(SectionSerialize::new)
                                        .collect::<Vec<_>>(),
                                }
                            }
                        }

                        let dashboard_content_serialize = DashboardContentSerialize::new(self);

                        async move { web::Response::ok_json(dashboard_content_serialize) }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                _ => async move { web::Response::error_404() }.boxed(),
            },
            uri_cursor::UriCursor::Next(section_index_str, uri_cursor) => {
                let section_index = match section_index_str.parse::<usize>() {
                    Ok(section_index) => section_index,
                    Err(_) => return async move { web::Response::error_404() }.boxed(),
                };

                let section = match self.sections.get(section_index) {
                    Some(section) => section,
                    None => return async move { web::Response::error_404() }.boxed(),
                };

                section.handle(request, uri_cursor)
            }
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}

#[derive(Debug)]
pub struct Section {
    name: Option<String>,

    content: SectionContent,
}
impl uri_cursor::Handler for Section {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        self.content.handle(request, uri_cursor)
    }
}

#[derive(Debug)]
pub enum SectionContent {
    Dashboards(SectionContentDashboards),
    Devices(SectionContentDevices),
}
impl uri_cursor::Handler for SectionContent {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match &self {
            SectionContent::Dashboards(section_content_dashboards) => {
                section_content_dashboards.handle(request, uri_cursor)
            }
            SectionContent::Devices(section_content_devices) => {
                section_content_devices.handle(request, uri_cursor)
            }
        }
    }
}

#[derive(Debug)]
pub struct SectionContentDashboards {
    dashboards: Vec<Dashboard>,
}
impl uri_cursor::Handler for SectionContentDashboards {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Next(dashboard_index_str, uri_cursor) => {
                let dashboard_index = match dashboard_index_str.parse::<usize>() {
                    Ok(dashboard_index) => dashboard_index,
                    Err(_) => return async move { web::Response::error_404() }.boxed(),
                };

                let dashboard = match self.dashboards.get(dashboard_index) {
                    Some(dashboard) => dashboard,
                    None => return async move { web::Response::error_404() }.boxed(),
                };

                dashboard.handle(request, uri_cursor)
            }
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}

#[derive(Debug)]
pub struct SectionContentDevices {
    device_ids: Vec<DeviceId>,
}
impl uri_cursor::Handler for SectionContentDevices {
    fn handle(
        &self,
        _request: web::Request,
        _uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        async move { web::Response::error_404() }.boxed()
    }
}

pub mod builder {
    use super::{super::fontawesome::Icon, *};
    use crate::devices::helpers::DeviceHandleErased;

    #[derive(Debug)]
    pub struct DashboardBuilder {
        name: String,
        icon: Icon,

        sections: Vec<Section>,
    }
    impl DashboardBuilder {
        pub fn new(
            name: String,
            icon: Icon,
        ) -> Self {
            let sections = Vec::<Section>::new();

            Self {
                name,
                icon,
                sections,
            }
        }

        pub fn section_add(
            &mut self,
            section: Section,
        ) {
            self.sections.push(section);
        }
        pub fn section_add_with(
            mut self,
            section: Section,
        ) -> Self {
            self.section_add(section);
            self
        }

        pub fn build(self) -> Dashboard {
            Dashboard {
                name: self.name,
                icon: self.icon,

                sections: self.sections,
            }
        }
    }

    #[derive(Debug)]
    pub struct SectionDashboardsBuilder {
        name: Option<String>,

        dashboards: Vec<Dashboard>,
    }
    impl SectionDashboardsBuilder {
        pub fn new(name: Option<String>) -> Self {
            let dashboards = Vec::<Dashboard>::new();

            Self { name, dashboards }
        }

        pub fn dashboard_add(
            &mut self,
            dashboard: Dashboard,
        ) {
            self.dashboards.push(dashboard);
        }
        pub fn dashboard_add_with(
            mut self,
            dashboard: Dashboard,
        ) -> Self {
            self.dashboard_add(dashboard);
            self
        }

        pub fn build(self) -> Section {
            Section {
                name: self.name,
                content: SectionContent::Dashboards(SectionContentDashboards {
                    dashboards: self.dashboards,
                }),
            }
        }
    }

    #[derive(Debug)]
    pub struct SectionDevicesBuilder {
        name: Option<String>,

        device_ids: Vec<DeviceId>,
    }
    impl SectionDevicesBuilder {
        pub fn new(name: Option<String>) -> Self {
            let device_ids = Vec::<DeviceId>::new();

            Self { name, device_ids }
        }

        pub fn device_add(
            &mut self,
            device: DeviceHandleErased<'_>,
        ) {
            self.device_ids.push(device.device_id());
        }
        pub fn device_add_with(
            mut self,
            device: DeviceHandleErased<'_>,
        ) -> Self {
            self.device_add(device);
            self
        }

        pub fn build(self) -> Section {
            Section {
                name: self.name,
                content: SectionContent::Devices(SectionContentDevices {
                    device_ids: self.device_ids,
                }),
            }
        }
    }
}
