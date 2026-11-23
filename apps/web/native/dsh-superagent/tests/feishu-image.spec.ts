import { expect, it } from 'vitest'
import { admissionFileKeys, admissionImageKeys } from '../src/sources/feishu-shared.ts'

it('admits only the image and file keys the handoff rendered into a message', () => {
  expect(admissionImageKeys('![Image](img_source) {"image_key":"img_second"} img_source')).toEqual(['img_source', 'img_second'])
  expect(admissionFileKeys('[文件] report.pdf file_v2_a1 [语音] 3s file_v2_b2 [视频] demo.mp4 file_v2_c3')).toEqual(['file_v2_a1', 'file_v2_b2', 'file_v2_c3'])
  // A key inside a word, or an image key, is not a file this Source may read.
  expect(admissionFileKeys('myfile_v2_a1 and [图片] img_v2_a1')).toEqual([])
})
