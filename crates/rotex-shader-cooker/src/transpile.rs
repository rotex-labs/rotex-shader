use rotex_shader_core::CompilerError;

pub fn transpile_spirv_to_wgsl(spirv_bytes: &[u8]) -> Result<String, CompilerError> {
    let module = naga::front::spv::parse_u8_slice(spirv_bytes, &naga::front::spv::Options::default())
        .map_err(|err| CompilerError::Transpile(err.to_string()))?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let info = validator
        .validate(&module)
        .map_err(|err| CompilerError::Transpile(err.to_string()))?;
    naga::back::wgsl::write_string(
        &module,
        &info,
        naga::back::wgsl::WriterFlags::empty(),
    )
    .map_err(|err| CompilerError::Transpile(err.to_string()))
}
