use std::{
    cell::{RefCell, OnceCell},
    rc::Rc
};
use gtk::{
    glib,
    prelude::*,
    subclass::prelude::*,
    CompositeTemplate,
};
use glib::{
    clone,
    Object,
    SignalHandlerId
};

use crate::{
    cache::Cache,
    common::{INode, INodeType}
};

use super::Library;


mod imp {
    use std::cell::Cell;

    use glib::{
        ParamSpec,
        ParamSpecString,
        ParamSpecEnum
    };
    use once_cell::sync::Lazy;
    use crate::common::INodeType;

    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/euphonica/Euphonica/gtk/library/folder-row.ui")]
    pub struct FolderRow {
        #[template_child]
        pub thumbnail: TemplateChild<gtk::Image>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub last_modified: TemplateChild<gtk::Label>,
        #[template_child]
        pub replace_queue: TemplateChild<gtk::Button>,
        #[template_child]
        pub append_queue: TemplateChild<gtk::Button>,
        pub uri: RefCell<String>,
        pub inode_type: Cell<INodeType>,
        // For unbinding the queue buttons when not bound to a song (i.e. being recycled)
        pub replace_queue_id: RefCell<Option<SignalHandlerId>>,
        pub append_queue_id: RefCell<Option<SignalHandlerId>>,
        // Only used while displaying a folder. For songs simply use a song MIME icon.
        pub thumbnail_signal_id: RefCell<Option<SignalHandlerId>>,
        pub library: OnceCell<Library>
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for FolderRow {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "EuphonicaFolderRow";
        type Type = super::FolderRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for FolderRow {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("uri").build(),
                    ParamSpecString::builder("last-modified").build(),
                    ParamSpecEnum::builder::<INodeType>("inode-type").build()
                ]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "uri" => self.uri.borrow().to_value(),
                "last-modified" => self.last_modified.label().to_value(),
                "inode-type" => self.inode_type.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "uri" => {
                    if let Ok(name) = value.get::<&str>() {
                        // Keep display name synchronised
                        if let Some(title) = name.split('/').last() {
                            self.title.set_label(title);
                        }
                        self.uri.replace(name.to_string());
                    }
                    else {
                        self.title.set_label("");
                    }
                }
                "last-modified" => {
                    if let Ok(lm) = value.get::<&str>() {
                        self.last_modified.set_label(lm);
                    }
                    else {
                        self.last_modified.set_label("");
                    }
                }
                "inode-type" => {
                    if let Ok(it) = value.get::<INodeType>() {
                        self.inode_type.replace(it);
                        self.thumbnail.set_icon_name(Some(it.icon_name()));
                        if it == INodeType::Folder || it == INodeType::Song {
                            self.replace_queue.set_visible(true);
                            self.append_queue.set_visible(true);
                        }
                        else {
                            self.replace_queue.set_visible(false);
                            self.append_queue.set_visible(false);
                        }
                        // TODO: playlists support
                    }
                    else {
                        self.thumbnail.set_icon_name(Some(&INodeType::default().icon_name()));
                        self.replace_queue.set_visible(false);
                        self.append_queue.set_visible(false);
                    }
                }
                _ => unimplemented!(),
            }
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for FolderRow {}

    // Trait shared by all boxes
    impl BoxImpl for FolderRow {}
}

glib::wrapper! {
    pub struct FolderRow(ObjectSubclass<imp::FolderRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl FolderRow {
    pub fn new(library: Library, item: &gtk::ListItem) -> Self {
        let res: Self = Object::builder().build();
        res.setup(library, item);
        res
    }

    #[inline(always)]
    pub fn setup(&self, library: Library, item: &gtk::ListItem) {
        let _ = self.imp().library.set(library);
        item
            .property_expression("item")
            .chain_property::<INode>("uri")
            .bind(self, "uri", gtk::Widget::NONE);

        item
            .property_expression("item")
            .chain_property::<INode>("last-modified")
            .bind(self, "last-modified", gtk::Widget::NONE);

        item
            .property_expression("item")
            .chain_property::<INode>("inode-type")
            .bind(self, "inode-type", gtk::Widget::NONE);

        // item
        //     .property_expression("item")
        //     .chain_property::<Song>("quality-grade")
        //     .bind(self, "quality-grade", gtk::Widget::NONE);
    }

    // fn update_thumbnail(&self, info: Option<&AlbumInfo>, cache: Rc<Cache>, schedule: bool) {
    //     if let Some(album) = info {
    //         // Should already have been downloaded by the album view
    //         if let Some(tex) = cache.load_cached_album_art(album, true, schedule) {
    //             self.imp().thumbnail.set_paintable(Some(&tex));
    //             return;
    //         }
    //     }
    //     self.imp().thumbnail.set_paintable(Some(&*ALBUMART_PLACEHOLDER));
    // }

    pub fn bind(&self, inode: &INode, _cache: Rc<Cache>) {
        // Bind album art listener. Set once first (like sync_create)
        // self.update_thumbnail(song.get_album(), cache.clone(), true);
        // let thumbnail_binding = cache.get_cache_state().connect_closure(
        //     "album-art-downloaded",
        //     false,
        //     closure_local!(
        //         #[weak(rename_to = this)]
        //         self,
        //         #[strong]
        //         song,
        //         #[weak]
        //         cache,
        //         move |_: CacheState, folder_uri: String| {
        //             if let Some(album) = song.get_album() {
        //                 if album.uri == folder_uri {
        //                     this.update_thumbnail(Some(album), cache, false);
        //                 }
        //             }
        //         }
        //     )
        // );
        // self.imp().thumbnail_signal_id.replace(Some(thumbnail_binding));
        // Bind the queue buttons
        let uri = inode.get_uri().to_owned();
        if let Some(old_id) = self.imp().replace_queue_id.replace(
            Some(
                self.imp().replace_queue.connect_clicked(
                    clone!(
                        #[weak(rename_to = this)]
                        self,
                        #[strong]
                        uri,
                        move |_| {
                            if let Some(library) = this.imp().library.get() {
                                match this.imp().inode_type.get() {
                                    INodeType::Song => {
                                        library.queue_uri(&uri, true, true, false);
                                    },
                                    INodeType::Folder => {
                                        library.queue_uri(&uri, true, true, true);
                                    },
                                    _ => unreachable!()
                                }
                            }
                        }
                    )
                )
            )
        ) {
            // Unbind old ID
            self.imp().replace_queue.disconnect(old_id);
        }
        if let Some(old_id) = self.imp().append_queue_id.replace(
            Some(
                self.imp().append_queue.connect_clicked(
                    clone!(
                        #[weak(rename_to = this)]
                        self,
                        #[strong]
                        uri,
                        move |_| {
                            if let Some(library) = this.imp().library.get() {
                                match this.imp().inode_type.get() {
                                    INodeType::Song => {
                                        library.queue_uri(&uri, false, false, false);
                                    },
                                    INodeType::Folder => {
                                        library.queue_uri(&uri, false, false, true);
                                    },
                                    _ => unreachable!()
                                }
                            }
                        }
                    )
                )
            )
        ) {
            // Unbind old ID
            self.imp().append_queue.disconnect(old_id);
        }
    }

    pub fn unbind(&self) {
        if let Some(id) = self.imp().replace_queue_id.borrow_mut().take() {
            self.imp().replace_queue.disconnect(id);
        }
        if let Some(id) = self.imp().append_queue_id.borrow_mut().take() {
            self.imp().append_queue.disconnect(id);
        }
    }
}