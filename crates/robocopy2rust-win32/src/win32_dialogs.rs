use std::path::PathBuf;
use winsafe::{self as w, co, prelude::*};

pub fn pick_folder(parent: &w::HWND, title: &str) -> w::AnyResult<Option<String>> {
    let dlg = w::CoCreateInstance::<w::IFileOpenDialog>(
        &co::CLSID::FileOpenDialog,
        None::<&w::IUnknown>,
        co::CLSCTX::INPROC_SERVER,
    )?;
    dlg.SetTitle(title)?;
    dlg.SetOptions(dlg.GetOptions()? | co::FOS::FORCEFILESYSTEM | co::FOS::PICKFOLDERS)?;
    if dlg.Show(parent)? {
        let item = dlg.GetResult()?;
        let path = item.GetDisplayName(co::SIGDN::FILESYSPATH)?;
        return Ok(Some(path));
    }
    Ok(None)
}

pub fn pick_open_file(parent: &w::HWND, title: &str, filters: &[(&str, &str)]) -> w::AnyResult<Option<PathBuf>> {
    let dlg = w::CoCreateInstance::<w::IFileOpenDialog>(
        &co::CLSID::FileOpenDialog,
        None::<&w::IUnknown>,
        co::CLSCTX::INPROC_SERVER,
    )?;
    dlg.SetTitle(title)?;
    dlg.SetOptions(dlg.GetOptions()? | co::FOS::FORCEFILESYSTEM | co::FOS::FILEMUSTEXIST)?;
    if !filters.is_empty() {
        dlg.SetFileTypes(filters)?;
        dlg.SetFileTypeIndex(1)?;
    }
    if dlg.Show(parent)? {
        let item = dlg.GetResult()?;
        let path = item.GetDisplayName(co::SIGDN::FILESYSPATH)?;
        return Ok(Some(PathBuf::from(path)));
    }
    Ok(None)
}

pub fn pick_save_file(
    parent: &w::HWND,
    title: &str,
    default_name: &str,
    default_ext: &str,
    filters: &[(&str, &str)],
) -> w::AnyResult<Option<PathBuf>> {
    let dlg = w::CoCreateInstance::<w::IFileSaveDialog>(
        &co::CLSID::FileSaveDialog,
        None::<&w::IUnknown>,
        co::CLSCTX::INPROC_SERVER,
    )?;
    dlg.SetTitle(title)?;
    dlg.SetOptions(dlg.GetOptions()? | co::FOS::FORCEFILESYSTEM | co::FOS::OVERWRITEPROMPT)?;
    if !filters.is_empty() {
        dlg.SetFileTypes(filters)?;
        dlg.SetFileTypeIndex(1)?;
    }
    if !default_name.is_empty() {
        dlg.SetFileName(default_name)?;
    }
    if !default_ext.is_empty() {
        dlg.SetDefaultExtension(default_ext)?;
    }
    if dlg.Show(parent)? {
        let item = dlg.GetResult()?;
        let path = item.GetDisplayName(co::SIGDN::FILESYSPATH)?;
        return Ok(Some(PathBuf::from(path)));
    }
    Ok(None)
}

pub fn copy_to_clipboard(text: &str) -> bool {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        return cb.set_text(text.to_string()).is_ok();
    }
    false
}
