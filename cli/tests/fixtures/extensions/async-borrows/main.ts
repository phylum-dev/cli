import * as PhylumApi from './phylum-api.ts'

await Promise.all([
  PhylumApi.get_user_info(),
  PhylumApi.get_user_info(),
  PhylumApi.get_user_info(),
  PhylumApi.get_user_info(),
  PhylumApi.get_user_info(),
  PhylumApi.get_user_info(),
  PhylumApi.get_user_info(),
  PhylumApi.get_user_info(),
  PhylumApi.get_user_info(),
])
