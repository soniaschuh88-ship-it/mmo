# Node.js gateway only (no Rust binary).
# Use when bifrost-server is running separately via BIFROST_SERVER_URL.
# For the full combined build (Rust + Node), use the root Dockerfile instead.

FROM node:20-slim
WORKDIR /app

COPY package*.json ./
RUN npm install --omit=dev

COPY . .
RUN chmod +x start.sh

ENV PORT=8080
ENV BIFROST_SERVER_URL=http://localhost:8080

EXPOSE 8080

CMD ["node", "index.js"]
