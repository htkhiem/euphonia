use std::{
    cell::{Cell, RefCell},
    rc::Rc
};
use gtk::{
    glib,
    prelude::*,
    subclass::prelude::*,
    CompositeTemplate,
    Label
};
use glib::{Object, Binding};

use crate::{
    common::Song,
    player::Player
};

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/slamprust/Slamprust/gtk/queue-row.ui")]
    pub struct QueueRow {
        #[template_child]
        pub song_name: TemplateChild<Label>,
        #[template_child]
        pub playing_indicator: TemplateChild<Label>,
        // Vector holding the bindings to properties of the Song GObject
        pub bindings: RefCell<Vec<Binding>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for QueueRow {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "SlamprustQueueRow";
        type Type = super::QueueRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for QueueRow {}

    // Trait shared by all widgets
    impl WidgetImpl for QueueRow {}

    // Trait shared by all boxes
    impl BoxImpl for QueueRow {}
}

glib::wrapper! {
    pub struct QueueRow(ObjectSubclass<imp::QueueRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}


impl Default for QueueRow {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind(&self, song: &Song) {
        // Get state
        let song_name_label = self.imp().song_name.get();
        let playing_label = self.imp().playing_indicator.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let song_name_binding = song
            .bind_property("name", &song_name_label, "label")
            .sync_create()
            .build();
        // Save binding
        bindings.push(song_name_binding);

        let song_is_playing_binding = song
            .bind_property("is-playing", &playing_label, "visible")
            .sync_create()
            .build();
        // Save binding
        bindings.push(song_is_playing_binding);
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}