//! .udf (Universal Disk Format — DVD/Blu-ray/đĩa quang, USB lớn) — chỉ giải nén, mở rộng
//! theo yêu cầu phủ danh sách định dạng 7-Zip hỗ trợ (xem CLAUDE.md mục "7-Zip feature
//! parity"). Trải qua 3 lần điều tra trong dự án này trước khi được thêm: lần 1/2 kết luận
//! "chưa có crate đủ tin cậy"; lần 3 (2026-08-13) xác nhận `hadris-udf` đã đủ chín — MIT,
//! pure Rust, không FFI, `no_std`-capable, phát triển liên tục từ 11/2024, bản 2.0.0 tại
//! thời điểm thêm, vẫn đang được bảo trì tích cực (push gần nhất chỉ vài ngày trước khi
//! thêm). Dùng bộ feature mặc định của crate (`std`+`read`+`sync`) — không cần bật thêm gì.
//!
//! **Giới hạn thật của crate, không phải lựa chọn có chủ đích ở đây**: `UdfVolume::read_file`
//! tải TOÀN BỘ nội dung 1 file vào `Vec<u8>` trong bộ nhớ — crate không có API kiểu
//! `open()`/`Read` streaming từng phần như `ext4-view` (dùng ở `ext_format.rs`) hay `rpm`
//! (dùng ở `rpm_format.rs`). UDF là định dạng cho đĩa quang/USB lớn — chính doc của crate nêu
//! rõ use case "Large USB drives (files >4GB)" — nên đây là sai lệch thật so với nguyên tắc
//! an toàn file lớn (DoD #4) áp dụng ở mọi nơi khác trong dự án, không phải đánh đổi được cân
//! nhắc trước. Không có cách né nếu không tự viết lại phần đọc allocation descriptor của
//! crate (rủi ro tương đương/cao hơn việc tự vá `sevenz-rust2`, đã từng bị từ chối) — ghi rõ
//! ở đây thay vì giả vờ đây là luồng streaming.
//!
//! UDF không có khái niệm mật khẩu/mã hoá — giống CAB/CPIO/LZH, không có tham số `password`.
//!
//! Khác ext2/ext3/ext4 (`ext_format.rs` phải tự lọc "."/".." để tránh đệ quy vô hạn), UDF
//! không có entry "." tự tham chiếu — mỗi thư mục chỉ liệt kê con thật + 1 entry ".." trỏ về
//! cha, và `UdfDir::entries()` của chính crate đã lọc bỏ ".." rồi (`is_parent()`).

use crate::{EntryInfo, Error, Result};
use hadris_udf::dir::UdfDirEntry;
use hadris_udf::{UdfDir, UdfVolume};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

fn map_err(archive_path: &Path, err: hadris_udf::Error) -> Error {
    Error::io(archive_path, std::io::Error::other(err.to_string()))
}

fn open(archive_path: &Path) -> Result<UdfVolume<BufReader<File>>> {
    let file = File::open(archive_path).map_err(|e| Error::io(archive_path, e))?;
    UdfVolume::open(BufReader::new(file)).map_err(|e| map_err(archive_path, e))
}

/// Duyệt đệ quy toàn bộ cây thư mục bắt đầu từ root — `hadris-udf` không có sẵn hàm "duyệt
/// hết" 1 lệnh, chỉ có `read_directory` cho từng thư mục con (theo ICB của nó).
fn walk(
    volume: &UdfVolume<BufReader<File>>,
    dir: &UdfDir,
    prefix: &str,
    archive_path: &Path,
    visit: &mut impl FnMut(String, &UdfDirEntry) -> Result<()>,
) -> Result<()> {
    for entry in dir.entries() {
        let rel = if prefix.is_empty() {
            entry.name().to_string()
        } else {
            format!("{prefix}/{}", entry.name())
        };
        if entry.is_dir() {
            visit(rel.clone(), entry)?;
            let sub = volume
                .read_directory(&entry.icb)
                .map_err(|e| map_err(archive_path, e))?;
            walk(volume, &sub, &rel, archive_path, visit)?;
        } else {
            visit(rel, entry)?;
        }
    }
    Ok(())
}

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let volume = open(archive_path)?;
    std::fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;

    let root = volume.root_dir().map_err(|e| map_err(archive_path, e))?;
    walk(&volume, &root, "", archive_path, &mut |rel, entry| {
        let out_path = dest_dir.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| Error::io(&out_path, e))
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            let bytes = volume
                .read_file(entry)
                .map_err(|e| map_err(archive_path, e))?;
            std::fs::write(&out_path, &bytes).map_err(|e| Error::io(&out_path, e))
        }
    })
}

pub fn list_entries(archive_path: &Path) -> Result<Vec<EntryInfo>> {
    let volume = open(archive_path)?;
    let mut entries = Vec::new();
    let root = volume.root_dir().map_err(|e| map_err(archive_path, e))?;
    walk(&volume, &root, "", archive_path, &mut |rel, entry| {
        entries.push(EntryInfo {
            name: rel,
            size: entry.size,
            is_dir: entry.is_dir(),
        });
        Ok(())
    })?;
    Ok(entries)
}

pub fn test_integrity(archive_path: &Path) -> Result<bool> {
    let volume = open(archive_path)?;
    let root = volume.root_dir().map_err(|e| map_err(archive_path, e))?;
    walk(&volume, &root, "", archive_path, &mut |_rel, entry| {
        if !entry.is_dir() {
            volume
                .read_file(entry)
                .map_err(|e| map_err(archive_path, e))?;
        }
        Ok(())
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// File không phải ảnh UDF thật -> phải báo lỗi rõ ràng qua đường public API, không
    /// panic. Không cần fixture UDF thật để xác nhận điều này.
    #[test]
    fn errors_clearly_on_garbage_file() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("garbage.udf");
        std::fs::write(&archive, b"day khong phai la 1 anh UDF that su").unwrap();

        let err = crate::list_entries(&archive, None).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }

    /// Ảnh UDF tối thiểu nhưng HỢP LỆ THẬT — không có fixture/writer nào ở thượng nguồn để
    /// mượn (`hadris-udf` chỉ có API đọc ở bản này, không có encoder). Thay vì tự đoán byte
    /// offset của từng struct (rủi ro cao — nhiều field `#[repr(C)]` có padding không hiển
    /// nhiên), cách dựng ở đây XÂY DỰNG THẲNG các struct THẬT của crate (`PrimaryVolumeDescriptor`,
    /// `PartitionDescriptor`, ...) rồi dùng `bytemuck::bytes_of` để lấy đúng byte thật — CHÍNH
    /// LÀ cách bản thân crate dùng để đọc lại (`bytemuck::from_bytes::<T>`), nên đảm bảo khớp
    /// tuyệt đối, không phụ thuộc vào việc tự tính offset đúng hay sai. Chỉ đúng trên máy
    /// little-endian (x86_64/ARM64 LE — mọi nền tảng dự án này build tới đều LE) vì bản thân
    /// crate cũng ngầm giả định điều này (gọi `.to_le()` sau khi `bytemuck::from_bytes`, vốn
    /// chỉ là no-op đúng nghĩa trên máy LE — xem `into_native()` ở `primary.rs`/`file.rs`...).
    /// Struct có field riêng tư (`reserved`) nên không literal-construct được — dùng
    /// `Zeroable::zeroed()` rồi chỉ gán field `pub` cần dùng, phần còn lại giữ 0 (an toàn, đã
    /// xác nhận qua đọc `fs.rs`: các field không set ở đây không được `read_vds`/`read_icb`
    /// dùng tới cho luồng mở archive + duyệt cây cơ bản).
    ///
    /// Riêng File Identifier Descriptor (FID) được `dir.rs::FileIdentifierDescriptor::from_bytes`
    /// đọc THỦ CÔNG theo offset cố định (không qua `bytemuck::from_bytes`) — giữ nguyên cách
    /// dựng bằng tay ở `build_root_fid`, các offset đã đối chiếu trực tiếp với hàm đó.
    fn tag_checksum(bytes: &[u8]) -> u8 {
        let mut sum: u8 = 0;
        for (i, &byte) in bytes.iter().enumerate() {
            if i != 4 {
                sum = sum.wrapping_add(byte);
            }
        }
        sum
    }

    fn make_tag(id: hadris_udf::descriptor::TagIdentifier, sector: u32, serial: u16) -> hadris_udf::descriptor::DescriptorTag {
        use hadris_udf::descriptor::DescriptorTag;
        let mut tag = DescriptorTag {
            tag_identifier: id.to_u16(),
            descriptor_version: 2,
            tag_serial_number: serial,
            tag_location: sector,
            ..Default::default()
        };
        tag.tag_checksum = tag_checksum(bytemuck::bytes_of(&tag));
        tag
    }

    fn write_struct<T: bytemuck::Pod>(data: &mut [u8], sector: u32, value: &T) {
        let off = sector as usize * 2048;
        let bytes = bytemuck::bytes_of(value);
        data[off..off + bytes.len()].copy_from_slice(bytes);
    }

    /// Dựng 1 ảnh UDF tối thiểu, hợp lệ thật, chứa đúng 1 file "hello.txt" ở thư mục gốc.
    /// Sơ đồ sector: 16-18 VRS, 256 AVDP, 257-260 Main VDS (PVD/PD/LVD/Terminating), partition
    /// bắt đầu từ sector 260 (tuyệt đối) — FSD ở partition-block 1 (=261 tuyệt đối), root ICB ở
    /// partition-block 2 (=262), file ICB "hello.txt" ở partition-block 4 (=264).
    fn build_minimal_udf(content: &[u8]) -> Vec<u8> {
        use bytemuck::Zeroable;
        use hadris_udf::descriptor::{
            AnchorVolumeDescriptorPointer, ExtentDescriptor, FileSetDescriptor,
            LogicalVolumeDescriptor, LongAllocationDescriptor, PartitionDescriptor,
            PrimaryVolumeDescriptor, TagIdentifier,
        };
        use hadris_udf::file::{FileEntry, IcbTag};

        let mut data = vec![0u8; 2048 * 270];

        // Volume Recognition Sequence — bắt buộc theo thứ tự BEA01/NSR02/TEA01.
        data[16 * 2048..16 * 2048 + 7].copy_from_slice(b"\0BEA01\x01");
        data[17 * 2048..17 * 2048 + 7].copy_from_slice(b"\0NSR02\x01");
        data[18 * 2048..18 * 2048 + 7].copy_from_slice(b"\0TEA01\x01");

        // Anchor Volume Descriptor Pointer tại sector 256 — main_vds_extent: 4 sector từ 257.
        let mut avdp: AnchorVolumeDescriptorPointer = Zeroable::zeroed();
        avdp.tag = make_tag(TagIdentifier::AnchorVolumeDescriptorPointer, 256, 1);
        avdp.main_vds_extent = ExtentDescriptor { length: 4 * 2048, location: 257 };
        write_struct(&mut data, 256, &avdp);

        // Primary Volume Descriptor tại sector 257.
        let mut pvd: PrimaryVolumeDescriptor = Zeroable::zeroed();
        pvd.tag = make_tag(TagIdentifier::PrimaryVolumeDescriptor, 257, 1);
        write_struct(&mut data, 257, &pvd);

        // Partition Descriptor tại sector 258 — partition bắt đầu từ sector tuyệt đối 260.
        let mut pd: PartitionDescriptor = Zeroable::zeroed();
        pd.tag = make_tag(TagIdentifier::PartitionDescriptor, 258, 1);
        pd.partition_starting_location = 260;
        pd.partition_length = 100;
        write_struct(&mut data, 258, &pd);

        // Logical Volume Descriptor tại sector 259 — block size 2048, file_set_location trỏ
        // tới block 1 TRONG partition (= sector tuyệt đối 261).
        let mut lvd: LogicalVolumeDescriptor = Zeroable::zeroed();
        lvd.tag = make_tag(TagIdentifier::LogicalVolumeDescriptor, 259, 1);
        lvd.logical_block_size = 2048;
        let fsd_loc = LongAllocationDescriptor {
            extent_length: 2048,
            logical_block_num: 1,
            partition_ref_num: 0,
            impl_use: [0; 6],
        };
        lvd.logical_volume_contents_use[..16].copy_from_slice(bytemuck::bytes_of(&fsd_loc));
        write_struct(&mut data, 259, &lvd);

        // Terminating Descriptor tại sector 260 — kết thúc Main VDS (đúng 4 sector 257-260).
        let term_tag = make_tag(TagIdentifier::TerminatingDescriptor, 260, 1);
        write_struct(&mut data, 260, &term_tag);

        // File Set Descriptor tại sector 261 (= partition block 1) — root_directory_icb trỏ
        // tới block 2 TRONG partition (= sector tuyệt đối 262). LƯU Ý: `read_file_set_descriptor`
        // validate tag bằng `icb.logical_block_num` (số TƯƠNG ĐỐI trong partition, = 1), KHÔNG
        // phải sector tuyệt đối 261 — khác hẳn PVD/PD/LVD/AVDP ở trên (dùng sector tuyệt đối) —
        // đã xác nhận qua đọc trực tiếp `fs.rs::read_file_set_descriptor`, không đoán.
        let mut fsd: FileSetDescriptor = Zeroable::zeroed();
        fsd.tag = make_tag(TagIdentifier::FileSetDescriptor, 1, 1);
        fsd.root_directory_icb = LongAllocationDescriptor {
            extent_length: 2048,
            logical_block_num: 2,
            partition_ref_num: 0,
            impl_use: [0; 6],
        };
        write_struct(&mut data, 261, &fsd);

        // Root directory File Entry (ICB) tại sector 262 (= partition block 2). AllocationType
        // = Embedded (bits 0-2 của icb_tag.flags = 3, xem file.rs::AllocationType::from_bits)
        // — dữ liệu FID nằm ngay trong chính sector ICB, sau FileEntry::BASE_SIZE (176 byte).
        // Tag location = 2 (số TƯƠNG ĐỐI trong partition, khớp FSD.root_directory_icb ở trên),
        // cùng lý do đã ghi ở FSD — `read_icb` validate bằng `icb.logical_block_num`, không
        // phải sector tuyệt đối 262.
        let fid_bytes = build_hello_fid();
        let mut root_icb: FileEntry = Zeroable::zeroed();
        root_icb.tag = make_tag(TagIdentifier::FileEntry, 2, 1);
        let mut root_icb_tag: IcbTag = Zeroable::zeroed();
        root_icb_tag.file_type = hadris_udf::FileType::Directory as u8;
        root_icb_tag.flags = 3; // Embedded
        root_icb.icb_tag = root_icb_tag;
        root_icb.information_length = fid_bytes.len() as u64;
        root_icb.extended_attributes_length = 0;
        root_icb.allocation_descriptors_length = fid_bytes.len() as u32;
        write_struct(&mut data, 262, &root_icb);
        let root_icb_off = 262 * 2048 + FileEntry::BASE_SIZE;
        data[root_icb_off..root_icb_off + fid_bytes.len()].copy_from_slice(&fid_bytes);

        // File Entry cho "hello.txt" tại sector 264 (= partition block 4) — nội dung nhúng
        // Embedded ngay sau FileEntry::BASE_SIZE, giống root ICB ở trên. Tag location = 4
        // (số tương đối, khớp FID.icb.logical_block_num dựng ở build_hello_fid), cùng lý do.
        let mut file_icb: FileEntry = Zeroable::zeroed();
        file_icb.tag = make_tag(TagIdentifier::FileEntry, 4, 1);
        let mut file_icb_tag: IcbTag = Zeroable::zeroed();
        file_icb_tag.file_type = hadris_udf::FileType::RegularFile as u8;
        file_icb_tag.flags = 3; // Embedded
        file_icb.icb_tag = file_icb_tag;
        file_icb.information_length = content.len() as u64;
        file_icb.extended_attributes_length = 0;
        file_icb.allocation_descriptors_length = content.len() as u32;
        write_struct(&mut data, 264, &file_icb);
        let file_icb_off = 264 * 2048 + FileEntry::BASE_SIZE;
        data[file_icb_off..file_icb_off + content.len()].copy_from_slice(content);

        data
    }

    /// FID cho "hello.txt", trỏ tới ICB tại partition-block 4 (= sector tuyệt đối 264). Đọc
    /// THỦ CÔNG bằng `FileIdentifierDescriptor::from_bytes` (không qua `bytemuck::from_bytes`)
    /// — offset dưới đây đối chiếu trực tiếp với đúng hàm đó ở `dir.rs`. Tag của FID chỉ được
    /// đọc `tag_identifier` (2 byte đầu) để xác định loại, KHÔNG qua `DescriptorTag::validate`
    /// (đã xác nhận đọc `from_bytes`) — nên checksum/version của tag này không quan trọng.
    fn build_hello_fid() -> Vec<u8> {
        let name = "hello.txt";
        // File Identifier CS0: byte đầu = compression id 8 (1 byte/ký tự — đủ cho ASCII),
        // theo đúng decode_filename() đã đọc ở dir.rs.
        let mut name_field = vec![8u8];
        name_field.extend_from_slice(name.as_bytes());

        let base_size = 38usize; // FileIdentifierDescriptor::BASE_SIZE, đã xác nhận ở dir.rs.
        let mut fid = vec![0u8; base_size];
        // tag_identifier (offset 0, 2 byte LE) = 257 (FileIdentifierDescriptor).
        let id = hadris_udf::descriptor::TagIdentifier::FileIdentifierDescriptor.to_u16();
        fid[0..2].copy_from_slice(&id.to_le_bytes());
        // file_version_number (offset 16, 2 byte) = 1.
        fid[16..18].copy_from_slice(&1u16.to_le_bytes());
        // file_characteristics (offset 18) = EXISTENCE (0x01) — không phải thư mục/parent.
        fid[18] = 0x01;
        // file_identifier_length (offset 19).
        fid[19] = name_field.len() as u8;
        // icb (offset 20, LongAllocationDescriptor 16 byte, đọc qua bytemuck::from_bytes ở
        // from_bytes()): extent_length(4)@20 + logical_block_num(4)@24 + partition_ref_num(2)@28
        // + impl_use(6)@30.
        let icb = hadris_udf::descriptor::LongAllocationDescriptor {
            extent_length: 2048,
            logical_block_num: 4,
            partition_ref_num: 0,
            impl_use: [0; 6],
        };
        fid[20..36].copy_from_slice(bytemuck::bytes_of(&icb));
        // implementation_use_length (offset 36, 2 byte) = 0.
        fid[36..38].copy_from_slice(&0u16.to_le_bytes());

        fid.extend_from_slice(&name_field);
        while fid.len() % 4 != 0 {
            fid.push(0);
        }

        fid
    }

    #[test]
    fn list_extract_and_test_roundtrip_on_minimal_real_image() {
        let content = b"xin chao tu UDF";
        let image = build_minimal_udf(content);

        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("test.udf");
        let mut f = std::fs::File::create(&archive).unwrap();
        f.write_all(&image).unwrap();
        drop(f);

        assert!(test_integrity(&archive).unwrap());

        let entries = list_entries(&archive).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "hello.txt");
        assert!(!entries[0].is_dir);
        assert_eq!(entries[0].size, content.len() as u64);

        let dest = tmp.path().join("out");
        extract(&archive, &dest).unwrap();
        let extracted = std::fs::read(dest.join("hello.txt")).unwrap();
        assert_eq!(extracted, content);
    }
}
