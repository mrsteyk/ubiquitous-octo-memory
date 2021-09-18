miniquad_add_plugin({
  register_plugin: function (importObject) {
    importObject.env.js_file_picker = function (a, b, c, d) {
      //wasm_exports.wasm_cb
      console.log(a, b, c, d)

      let accept = '.bsp'
      if (d == 1) {
        accept = '.json'
      }

      var input = document.createElement('input')
      input.type = 'file'
      input.accept = accept

      input.onchange = (e) => {
        var file = e.target.files[0]
        var reader = new FileReader()
        reader.readAsArrayBuffer(file)

        reader.onload = function (e) {
          var arrayBuffer = e.target.result
          var bytes = new Uint8Array(arrayBuffer)
          console.log(file.name, bytes)

          // const buf = new Uint8Array(wasm_memory.buffer)
          const text = new TextEncoder('utf-8')

          const stem = text.encode(
            file.name.substring(0, file.name.length - accept.length)
          )
          const stem_len = stem.length
          const stem_buf = wasm_exports.malloc(stem_len)
          const data_buf = wasm_exports.malloc(bytes.length)

          console.log({ stem_buf, data_buf })

          getArray(stem_buf, Uint8Array, stem_len).set(stem)
          getArray(data_buf, Uint8Array, bytes.length).set(bytes)

          wasm_exports.wasm_cb(
            a,
            b,
            c,
            d,
            stem_buf,
            stem_len,
            data_buf,
            bytes.length
          )
        }
      }

      input.click()
    }

    importObject.env.js_save_file = function (
      name_data,
      name_len,
      data_data,
      data_len,
      ext_data,
      ext_len
    ) {
      const text = new TextDecoder()
      const name_buf = getArray(name_data, Uint8Array, name_len)
      const data_buf = getArray(data_data, Uint8Array, data_len)
      const ext_buf = getArray(ext_data, Uint8Array, ext_len)

      const name = text.decode(name_buf)
      const ext = text.decode(ext_buf)

      const blob = new Blob([data_buf], { type: 'application/octet-binary' })
      const url = URL.createObjectURL(blob)

      const a = document.createElement('a')
      a.href = url
      a.download = `${name}.${ext}`

      document.body.appendChild(a)
      a.click()
      URL.revokeObjectURL(url)
      document.body.removeChild(a)
    }
  },

  name: 'k0k',
  version: '0.0.1',
})
