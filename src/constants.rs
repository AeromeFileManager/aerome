use std::fs;

pub const APP_NAME: &'static str = "Future";

pub const APP_ICON_PLACES_FOLDER: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder.png");

pub const APP_ICON_PLACES_FOLDER_PICTURES: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-pictures.png");

pub const APP_ICON_PLACES_FOLDER_DOCUMENTS: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-documents.png");

pub const APP_ICON_PLACES_FOLDER_DOWNLOAD: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-download.png");

pub const APP_ICON_PLACES_FOLDER_DROPBOX: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-dropbox.png");

pub const APP_ICON_PLACES_FOLDER_MUSIC: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-music.png");

pub const APP_ICON_PLACES_FOLDER_PUBLICSHARE: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-publicshare.png");

pub const APP_ICON_PLACES_FOLDER_REMOTE: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-remote.png");

pub const APP_ICON_PLACES_FOLDER_TEMPLATES: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-templates.png");

pub const APP_ICON_PLACES_FOLDER_VIDEOS: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-videos.png");

pub const APP_ICON_PLACES_FOLDER_INSYNC: &'static [u8] =
    include_bytes!("../assets/Yaru/places/insync-folder.png");

pub const APP_ICON_PLACES_FOLDER_NETWORK_SERVER: &'static [u8] =
    include_bytes!("../assets/Yaru/places/network-server.png");

pub const APP_ICON_PLACES_FOLDER_NETWORK_WORKGROUP: &'static [u8] =
    include_bytes!("../assets/Yaru/places/network-workgroup.png");

pub const APP_ICON_PLACES_FOLDER_USER_DESKTOP: &'static [u8] =
    include_bytes!("../assets/Yaru/places/user-desktop.png");

pub const APP_ICON_PLACES_FOLDER_USER_HOME: &'static [u8] =
    include_bytes!("../assets/Yaru/places/user-home.png");

pub const APP_ICON_PLACES_FOLDER_USER_TRASH: &'static [u8] =
    include_bytes!("../assets/Yaru/places/user-trash.png");

pub const APP_ICON_SCALABLE_ERROR_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/error-symbolic.svg");

pub const APP_ICON_SCALABLE_GO_DOWN_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/go-down-symbolic.svg");

pub const APP_ICON_SCALABLE_IMAGE_MISSING_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/image-missing-symbolic.svg");

pub const APP_ICON_SCALABLE_OPEN_MENU_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/open-menu-symbolic.svg");

pub const APP_ICON_SCALABLE_STOPWATCH_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/stopwatch-symbolic.svg");

pub const APP_ICON_SCALABLE_VIEW_SORT_ASCENDING_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/view-sort-ascending-symbolic.svg");

pub const APP_ICON_SCALABLE_VIEW_SORT_DESCENDING_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/view-sort-descending-symbolic.svg");

pub const APP_ICON_SCALABLE_WINDOW_CLOSE_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/window-close-symbolic.svg");

pub const APP_ICON_SCALABLE_WINDOW_MAXIMIZE_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/window-maximize-symbolic.svg");

pub const APP_ICON_SCALABLE_WINDOW_MINIMIZE_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/window-minimize-symbolic.svg");

pub fn install() {
    let icons_dir = dirs::data_local_dir()
        .map(|data_dir| data_dir.join(APP_NAME).join("Yaru"))
        .expect("Could not find the apps data directory");

    println!("{icons_dir:?}");
    let places_dir = icons_dir.join("places");
    let scalable_dir = icons_dir.join("scalable");

    fs::create_dir_all(&icons_dir).expect("Could not write to the apps data directory");
    fs::create_dir_all(&places_dir).unwrap();
    fs::create_dir_all(&scalable_dir).unwrap();

    if places_dir.join("./folder.png").exists() {
        return;
    }

    fs::write(
        places_dir.join("./folder.png"),
        APP_ICON_PLACES_FOLDER).unwrap();

    fs::write(
        places_dir.join("./folder-pictures.png"),
        APP_ICON_PLACES_FOLDER_PICTURES).unwrap();

    fs::write(
        places_dir.join("./folder-documents.png"),
        APP_ICON_PLACES_FOLDER_DOCUMENTS).unwrap();

    fs::write(
        places_dir.join("./folder-download.png"),
        APP_ICON_PLACES_FOLDER_DOWNLOAD).unwrap();

    fs::write(
        places_dir.join("./folder-dropbox.png"),
        APP_ICON_PLACES_FOLDER_DROPBOX).unwrap();

    fs::write(
        places_dir.join("./folder-music.png"),
        APP_ICON_PLACES_FOLDER_MUSIC).unwrap();

    fs::write(
        places_dir.join("./folder-pictures.png"),
        APP_ICON_PLACES_FOLDER_PICTURES).unwrap();

    fs::write(
        places_dir.join("./folder-publicshare.png"),
        APP_ICON_PLACES_FOLDER_PUBLICSHARE).unwrap();

    fs::write(
        places_dir.join("./folder-remote.png"),
        APP_ICON_PLACES_FOLDER_REMOTE).unwrap();

    fs::write(
        places_dir.join("./folder-templates.png"),
        APP_ICON_PLACES_FOLDER_TEMPLATES).unwrap();

    fs::write(
        places_dir.join("./folder-videos.png"),
        APP_ICON_PLACES_FOLDER_VIDEOS).unwrap();

    fs::write(
        places_dir.join("./insync-folder.png"),
        APP_ICON_PLACES_FOLDER_INSYNC).unwrap();

    fs::write(
        places_dir.join("./network-server.png"),
        APP_ICON_PLACES_FOLDER_NETWORK_SERVER).unwrap();

    fs::write(
        places_dir.join("./network-workgroup.png"),
        APP_ICON_PLACES_FOLDER_NETWORK_WORKGROUP).unwrap();

    fs::write(
        places_dir.join("./user-desktop.png"),
        APP_ICON_PLACES_FOLDER_USER_DESKTOP).unwrap();

    fs::write(
        places_dir.join("./user-home.png"),
        APP_ICON_PLACES_FOLDER_USER_HOME).unwrap();

    fs::write(
        places_dir.join("./user-trash.png"),
        APP_ICON_PLACES_FOLDER_USER_TRASH).unwrap();

    fs::write(
        scalable_dir.join("./error-symbolic.svg"),
        APP_ICON_SCALABLE_ERROR_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./go-down-symbolic.svg"),
        APP_ICON_SCALABLE_GO_DOWN_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./image-missing-symbolic.svg"),
        APP_ICON_SCALABLE_IMAGE_MISSING_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./open-menu-symbolic.svg"),
        APP_ICON_SCALABLE_OPEN_MENU_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./stopwatch-symbolic.svg"),
        APP_ICON_SCALABLE_STOPWATCH_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./view-sort-ascending-symbolic.svg"),
        APP_ICON_SCALABLE_VIEW_SORT_ASCENDING_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./view-sort-descending-symbolic.svg"),
        APP_ICON_SCALABLE_VIEW_SORT_DESCENDING_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./window-close-symbolic.svg"),
        APP_ICON_SCALABLE_WINDOW_CLOSE_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./window-maximize-symbolic.svg"),
        APP_ICON_SCALABLE_WINDOW_MAXIMIZE_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./window-minimize-symbolic.svg"),
        APP_ICON_SCALABLE_WINDOW_MINIMIZE_SYMBOLIC).unwrap();

}
