# Cisco Camera Proxy

Cisco CIVS-IPC-7070 cameras have an RTSP server, but they use a proprietary
HTTPS-based authentication method. While this is better since it uses TLS,
it doesn't integrate with any open-source projects that I have found.

This is a small proxy program to allow the use of a hard-coded URL that gets
redirected to a session-specific URL upon connection.

Simply compile and run this server, then point your NVR at
`rtsp://[your_ip_here]:5554/[ip_of_camera]/[camera_username]/[camera_password]`.

This server will use the path segments to authenticate with the camera, generate
a session ID, create a new URL for the stream, and redirect the client to that URL.

## Error handling

Please don't rip me a new one for my error handling. I know; I just don't need it,
so I didn't want to invest the time. Feel free to open a PR if you do. The code
handles errors by silently dropping the connection, but this is a set-it-and-forget-it
system that you shouldn't have to debug outside initial setup.
