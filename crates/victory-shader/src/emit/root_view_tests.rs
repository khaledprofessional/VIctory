//! End-to-end tests of scalar loads (`s_load_dword*`) from a root view's
//! address: synthetic GFX10 programs lowered to DXIL, validated by
//! `dxil.dll` when present, and run on the hardware GPU and on WARP.

use super::host::{Dispatch, Host, Root, View};
use super::{HostShader, lower, validator};
use crate::native::{
    ComputeIo, HwStage, NativeProgram, ResourceType, RootParam, RootType, StageIo, UsedResource,
};
use crate::{io, ir, resource, structure};

const S_ENDPGM: u32 = 0xBF81_0000;

/// A root view in user SGPRs 0-1 (root parameter 0) and a table in user
/// SGPR 2 (root parameter 1) whose entry 0 is a raw UAV: dwords 2 and 3
/// of the view's memory go to dwords 0 and 1 of the UAV.
const READ_DWORDS_2_AND_3: &[u32] = &[
    0xF404_0100,
    0xFA00_0008, // s_load_dwordx2 s[4:5], s[0:1], 0x8
    0xBE86_0302, // s_mov_b32 s6, s2
    0xBE87_0380, // s_mov_b32 s7, 0
    0xF408_0203,
    0xFA00_0000, // s_load_dwordx4 s[8:11], s[6:7], null
    0xBF8C_C07F, // s_waitcnt lgkmcnt(0)
    0x7E02_0204, // v_mov_b32 v1, s4
    0x7E04_0205, // v_mov_b32 v2, s5
    0xE070_0000,
    0x8002_0100, // buffer_store_dword v1, off, s[8:11], 0
    0xE070_0004,
    0x8002_0200, // buffer_store_dword v2, off, s[8:11], 0 offset:4
    S_ENDPGM,
];

/// [`READ_DWORDS_2_AND_3`] lowered with the root view of `kind`.
fn build(kind: ResourceType) -> HostShader {
    let param = |root, sgprs, user_data, slot| RootParam {
        root,
        table_offset_dwords: 0,
        sgprs,
        base_register: 0,
        user_data,
        slot,
    };
    let used = |kind, root, user_data, slot| UsedResource {
        kind,
        root,
        table_offset_dwords: 0,
        lower_bound: 0,
        stride: 0,
        dwords: 4,
        user_data,
        slot,
    };
    let native = NativeProgram {
        stage: HwStage::Cs,
        wave64: false,
        vgprs: 8,
        layout: vec![
            param(RootType::Descriptor, 2, 0, 0),
            param(RootType::Table, 1, 2, 1),
        ],
        used: vec![
            used(kind, RootType::Descriptor, 0, 0),
            used(ResourceType::Uav, RootType::Table, 2, 1),
        ],
        io: StageIo::Cs(ComputeIo {
            rsrc2: 3 << 1,
            threads: [1, 1, 1],
        }),
        code: READ_DWORDS_2_AND_3
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect(),
    };
    let p = ir::build(&native.code, native.wave64, native.vgprs).expect("ir::build");
    let names: Vec<_> = p
        .blocks
        .iter()
        .flat_map(|b| b.insts.iter().map(|i| i.name()))
        .collect();
    assert_eq!(
        names,
        [
            "s_load_dwordx2",
            "s_mov_b32",
            "s_mov_b32",
            "s_load_dwordx4",
            "v_mov_b32",
            "v_mov_b32",
            "buffer_store_dword",
            "buffer_store_dword",
            "s_endpgm",
        ]
    );
    let st = structure::structure(&p).expect("structure");
    let res = resource::track(&p, &native.layout, native.stage, Some(&st)).expect("track");
    let io = io::raise(&p, &native).expect("io::raise");
    let lowered = lower::lower(&lower::Parts {
        native: &native,
        p: &p,
        st: &st,
        io: &io,
        res: &res,
        inputs: Vec::new(),
        outputs: Vec::new(),
        prims: Vec::new(),
        mesh_stride: 0,
        options: super::TranslateOptions::default(),
    })
    .expect("lower");
    let shader = HostShader {
        stage: HwStage::Cs,
        dxil: lowered.dxil,
        user_data: lowered.user_data,
        samplers: lowered.samplers,
        static_samplers: Vec::new(),
        root_cbvs: lowered.root_cbvs,
        root_srvs: lowered.root_srvs,
    };
    if let Ok(mut v) = validator::Validator::load(validator::Validator::default_path().as_deref()) {
        v.validate(&mut shader.dxil.clone())
            .expect("DXIL validation failed");
    } else {
        eprintln!("no dxil.dll; validation skipped");
    }
    shader
}

#[test]
fn scalar_loads_read_a_root_srv_and_a_root_cbv() {
    let _one = super::valu_tests::ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let data: Vec<u32> = (0..64).map(|i| 0x2000 + i).collect();
    for (kind, srv) in [(ResourceType::Srv, true), (ResourceType::Cbv, false)] {
        let shader = build(kind);
        assert_eq!(
            (shader.root_srvs.len(), shader.root_cbvs.len()),
            if srv { (1, 0) } else { (0, 1) },
            "{kind:?}: the root view the shader declares"
        );
        for warp in [false, true] {
            let host = match Host::new(warp) {
                Ok(h) => h,
                Err(e) if warp => {
                    eprintln!("no SM 6.6 WARP device ({e}); skipped");
                    continue;
                }
                Err(e) => panic!("no SM 6.6 hardware device: {e}"),
            };
            let label = if warp { "WARP" } else { "hardware" };
            let mut ud = vec![0u32; shader.user_data_dwords().max(1) as usize];
            for u in &shader.user_data {
                assert_eq!(u.root, RootType::Table, "{u:?}");
                ud[u.index as usize] = 0;
            }
            let (root, pso) = host
                .compute_pipeline(&shader)
                .unwrap_or_else(|e| panic!("{label} {kind:?}: {e}"));
            let out = host
                .dispatch(
                    &root,
                    &pso,
                    &Dispatch {
                        roots: vec![
                            (0, Root::Constants(ud)),
                            (
                                1,
                                Root::View {
                                    srv,
                                    data: data.clone(),
                                },
                            ),
                        ],
                        views: &[View::RawUav(2)],
                        samplers: &[],
                        groups: [1, 1, 1],
                    },
                )
                .unwrap_or_else(|e| panic!("{label} {kind:?}: {e}"));
            assert_eq!(out[0], [0x2002, 0x2003], "{label} {kind:?}");
            eprintln!("{label}: scalar loads from a root {kind:?} exact");
        }
    }
}
