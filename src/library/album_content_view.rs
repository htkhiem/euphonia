use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};
use time::{Date, format_description};
use adw::subclass::prelude::*;
use gtk::{
    prelude::*,
    gio,
    glib,
    CompositeTemplate,
    SignalListItemFactory,
    ListItem,
};
use glib::{
    clone,
    closure_local,
    Binding,
    signal::SignalHandlerId
};

use super::{
    Library,
    AlbumSongRow
};
use crate::{
    utils::format_secs_as_duration,
    common::{Album, Song},
    cache::{
        Cache,
        CacheState,
        placeholders::ALBUMART_PLACEHOLDER
    },
    client::ClientState
};

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/org/euphonia/Euphonia/gtk/library/album-content-view.ui")]
    pub struct AlbumContentView {
        #[template_child]
        pub infobox_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub collapse_infobox: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub cover: TemplateChild<gtk::Image>,
        #[template_child]
        pub content: TemplateChild<gtk::ListView>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist: TemplateChild<gtk::Label>,

        #[template_child]
        pub wiki_box: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub wiki_text: TemplateChild<gtk::Label>,
        #[template_child]
        pub wiki_link: TemplateChild<gtk::LinkButton>,
        #[template_child]
        pub wiki_attrib: TemplateChild<gtk::Label>,

        #[template_child]
        pub release_date: TemplateChild<gtk::Label>,
        #[template_child]
        pub track_count: TemplateChild<gtk::Label>,
        #[template_child]
        pub runtime: TemplateChild<gtk::Label>,

        #[template_child]
        pub replace_queue: TemplateChild<gtk::Button>,
        #[template_child]
        pub append_queue: TemplateChild<gtk::Button>,

        pub song_list: gio::ListStore,

        pub album: RefCell<Option<Album>>,
        pub bindings: RefCell<Vec<Binding>>,
        pub cover_signal_id: RefCell<Option<SignalHandlerId>>,
        pub cache: OnceCell<Rc<Cache>>,
    }

    impl Default for AlbumContentView {
        fn default() -> Self {
            Self {
                cover: TemplateChild::default(),
                title: TemplateChild::default(),
                artist: TemplateChild::default(),
                release_date: TemplateChild::default(),
                track_count: TemplateChild::default(),
                infobox_revealer: TemplateChild::default(),
                collapse_infobox: TemplateChild::default(),
                wiki_box: TemplateChild::default(),
                wiki_text: TemplateChild::default(),
                wiki_link: TemplateChild::default(),
                wiki_attrib: TemplateChild::default(),
                runtime: TemplateChild::default(),
                content: TemplateChild::default(),
                song_list: gio::ListStore::new::<Song>(),
                replace_queue: TemplateChild::default(),
                append_queue: TemplateChild::default(),
                album: RefCell::new(None),
                bindings: RefCell::new(Vec::new()),
                cover_signal_id: RefCell::new(None),
                cache: OnceCell::new()
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumContentView {
        const NAME: &'static str = "EuphoniaAlbumContentView";
        type Type = super::AlbumContentView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.set_layout_manager_type::<gtk::BinLayout>();
            // klass.set_css_name("albumview");
            klass.set_accessible_role(gtk::AccessibleRole::Group);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AlbumContentView {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for AlbumContentView {}
}

glib::wrapper! {
    pub struct AlbumContentView(ObjectSubclass<imp::AlbumContentView>)
        @extends gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for AlbumContentView {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl AlbumContentView {
    fn update_meta(&self, album: &Album) {
        let cache = self.imp().cache.get().unwrap().clone();
        let wiki_box = self.imp().wiki_box.get();
        let wiki_text = self.imp().wiki_text.get();
        let wiki_link = self.imp().wiki_link.get();
        let wiki_attrib = self.imp().wiki_attrib.get();
        if let Some(meta) = cache.load_local_album_meta(
            album.get_mbid().as_deref(),
            Some(album.get_title()).as_deref(),
            album.get_artist_str().as_deref()
        ) {
            if let Some(wiki) = meta.wiki {
                wiki_box.set_visible(true);
                wiki_text.set_label(&wiki.content);
                if let Some(url) = wiki.url.as_ref() {
                    wiki_link.set_visible(true);
                    wiki_link.set_uri(url);
                }
                else {
                    wiki_link.set_visible(false);
                }
                wiki_attrib.set_label(&wiki.attribution);
            }
            else {
                wiki_box.set_visible(false);
            }
        }
        else {
            wiki_box.set_visible(false);
        }
    }

    pub fn setup(&self, library: Library, cache: Rc<Cache>, client_state: ClientState) {
        cache.get_cache_state().connect_closure(
            "album-art-downloaded",
            false,
            closure_local!(
                #[weak(rename_to = this)]
                self,
                move |_: CacheState, folder_uri: String| {
                    if let Some(album) = this.imp().album.borrow().as_ref() {
                        if folder_uri == album.get_uri() {
                            this.update_cover(&folder_uri);
                        }
                    }
                }
            )
        );
        cache.get_cache_state().connect_closure(
            "album-meta-downloaded",
            false,
            closure_local!(
                #[weak(rename_to = this)]
                self,
                move |_: CacheState, folder_uri: String| {
                    if let Some(album) = this.imp().album.borrow().as_ref() {
                        if folder_uri == album.get_uri() {
                            this.update_meta(album);
                        }
                    }
                }
            )
        );
        client_state.connect_closure(
            "album-songs-downloaded",
            false,
            closure_local!(
                #[weak(rename_to = this)]
                self,
                move |_: ClientState, tag: String, songs: glib::BoxedAnyObject| {
                    if let Some(album) = this.imp().album.borrow().as_ref() {
                        if album.get_title() == tag {
                            this.add_songs(songs.borrow::<Vec<Song>>().as_ref());
                        }
                    }
                }
            )
        );

        let _ = self.imp().cache.set(cache);
        let infobox_revealer = self.imp().infobox_revealer.get();
        let collapse_infobox = self.imp().collapse_infobox.get();
        collapse_infobox
            .bind_property(
                "active",
                &infobox_revealer,
                "reveal-child"
            )
            .transform_to(|_, active: bool| { Some(!active) })
            .transform_from(|_, active: bool| { Some(!active) })
            .bidirectional()
            .sync_create()
            .build();

        infobox_revealer
            .bind_property(
                "child-revealed",
                &collapse_infobox,
                "icon-name"
            )
            .transform_to(|_, revealed| {
                if revealed {
                    return Some("up-symbolic");
                }
                Some("down-symbolic")
            })
            .sync_create()
            .build();

        let replace_queue_btn = self.imp().replace_queue.get();
        replace_queue_btn.connect_clicked(
            clone!(
                #[strong(rename_to = this)]
                self,
                #[weak]
                library,
                move |_| {
                    if let Some(album) = this.imp().album.borrow().as_ref() {
                        library.queue_album(album.clone(), true, true);
                    }
                }
            )
        );
        let append_queue_btn = self.imp().append_queue.get();
        append_queue_btn.connect_clicked(
            clone!(
                #[strong(rename_to = this)]
                self,
                #[weak]
                library,
                move |_| {
                    if let Some(album) = this.imp().album.borrow().as_ref() {
                        library.queue_album(album.clone(), false, false);
                    }
                }
            )
        );

        // Set up factory
        let factory = SignalListItemFactory::new();

        // Create an empty `AlbumSongRow` during setup
        factory.connect_setup(move |_, list_item| {
            let item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem");
            let row = AlbumSongRow::new(library.clone(), &item);
            item.set_child(Some(&row));
        });
        // Tell factory how to bind `AlbumSongRow` to one of our Album GObjects
        factory.connect_bind(move |_, list_item| {
            // Get `Song` from `ListItem` (that is, the data side)
            let item: Song = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<Song>()
                .expect("The item has to be a common::Song.");

            // Get `AlbumSongRow` from `ListItem` (the UI widget)
            let child: AlbumSongRow = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<AlbumSongRow>()
                .expect("The child has to be an `AlbumSongRow`.");

            // Within this binding fn is where the cached album art texture gets used.
            child.bind(&item);
        });


        // When row goes out of sight, unbind from item to allow reuse with another.
        factory.connect_unbind(move |_, list_item| {
            // Get `AlbumSongRow` from `ListItem` (the UI widget)
            let child: AlbumSongRow = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<AlbumSongRow>()
                .expect("The child has to be an `AlbumSongRow`.");
            child.unbind();
        });

        // Set the factory of the list view
        self.imp().content.set_factory(Some(&factory));
    }

    /// Returns true if an album art was successfully retrieved.
    /// On false, we will want to call cache.ensure_local_album_art()
    fn update_cover(&self, folder_uri: &str) -> bool {
        if let Some(cache) = self.imp().cache.get() {
            if let Some(tex) = cache.load_local_album_art(folder_uri, false) {
                self.imp().cover.set_paintable(Some(&tex));
                return true;
            }
            else {
                self.imp().cover.set_paintable(Some(&*ALBUMART_PLACEHOLDER));
                return false;
            }
        }
        false
    }

    pub fn bind(&self, album: Album) {
        println!("Binding to album: {:?}", &album);
        let title_label = self.imp().title.get();
        let artist_label = self.imp().artist.get();
        let release_date_label = self.imp().release_date.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let title_binding = album
            .bind_property("title", &title_label, "label")
            .sync_create()
            .build();
        // Save binding
        bindings.push(title_binding);

        let artist_binding = album
            .bind_property("artist", &artist_label, "label")
            .sync_create()
            .build();
        // Save binding
        bindings.push(artist_binding);

        self.update_meta(&album);
        let release_date_binding = album
            .bind_property("release_date", &release_date_label, "label")
            .transform_to(
                |_, boxed_date: glib::BoxedAnyObject| {
                    let format = format_description::parse("[year]-[month]-[day]").ok().unwrap();
                    if let Some(release_date) = boxed_date.borrow::<Option<Date>>().as_ref() {
                        return release_date.format(
                            &format
                        ).ok();
                    }
                    Some("-".to_owned())
                }
            )
            .sync_create()
            .build();
        // Save binding
        bindings.push(release_date_binding);

        let release_date_viz_binding = album
            .bind_property("release_date", &release_date_label, "visible")
            .transform_to(
                |_, boxed_date: glib::BoxedAnyObject| {
                    if boxed_date.borrow::<Option<Date>>().is_some() {
                        return Some(true);
                    }
                    Some(false)
                }
            )
            .sync_create()
            .build();
        // Save binding
        bindings.push(release_date_viz_binding);

        let uri = album.get_uri();
        if !self.update_cover(uri) {
            if let Some(cache) = self.imp().cache.get() {
                cache.ensure_local_album_art(uri);
            }
        }

        // Save reference to album object
        let sel_model = gtk::NoSelection::new(Some(self.imp().song_list.clone()));
        self.imp().content.set_model(Some(&sel_model));
        self.imp().album.borrow_mut().replace(album);
    }

    pub fn unbind(&self) {
        println!("Album content page hidden. Unbinding...");
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
        if let Some(id) = self.imp().cover_signal_id.take() {
            if let Some(cache) = self.imp().cache.get() {
                cache.get_cache_state().disconnect(id);
            }
        }
        // Unset metadata widgets
        self.imp().wiki_box.set_visible(false);
        self.imp().song_list.remove_all();
    }

    fn add_songs(&self, songs: &[Song]) {
        self.imp().song_list.extend_from_slice(songs);
        self.imp().track_count.set_label(&self.imp().song_list.n_items().to_string());
        self.imp().runtime.set_label(
            &format_secs_as_duration(
                self.imp().song_list
                    .iter()
                    .map(
                        |item: Result<Song, _>| {
                            if let Ok(song) = item {
                                return song.get_duration();
                            }
                            0
                        }
                    )
                    .sum::<u64>() as f64
            )
        );
    }
}
