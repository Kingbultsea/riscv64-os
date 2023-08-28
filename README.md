curl 'https://bfe.ewt360.com/video/playbackProgress' \
  -H 'sec-ch-ua: "Chromium";v="110", "Not A(Brand";v="24", "Google Chrome";v="110"' \
  -H 'sec-ch-ua-platform: "macOS"' \
  -H 'Referer: https://teacher.ewt360.com/' \
  -H 'sec-ch-ua-mobile: ?0' \
  -H 'User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36' \
  -H 'Content-Type: multipart/form-data; boundary=----WebKitFormBoundaryGaZRQDB3p6UAtLB2' \
  --data-raw $'------WebKitFormBoundaryGaZRQDB3p6UAtLB2\r\nContent-Disposition: form-data; name="userId"\r\n\r\n145462093\r\n------WebKitFormBoundaryGaZRQDB3p6UAtLB2\r\nContent-Disposition: form-data; name="lessonId"\r\n\r\n65068\r\n------WebKitFormBoundaryGaZRQDB3p6UAtLB2\r\nContent-Disposition: form-data; name="playedProgress"\r\n\r\n406.722075\r\n------WebKitFormBoundaryGaZRQDB3p6UAtLB2--\r\n' \
  --compressed