CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Auto-update timestamp function
CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- System users
CREATE TABLE users (
                       id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                       username TEXT UNIQUE NOT NULL,
                       password_hash TEXT NOT NULL,
                       created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER set_timestamp_users
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

-- Logical containers for books
CREATE TABLE libraries (
                           id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                           name TEXT NOT NULL,
                           description TEXT,
                           owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                           created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                           updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER set_timestamp_libraries
    BEFORE UPDATE ON libraries
    FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

-- Universal book metadata and physical properties
CREATE TABLE books (
                       id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                       library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
                       catalog_number SERIAL UNIQUE,
                       isbn_13 TEXT,
                       isbn_10 TEXT,
                       open_library_id TEXT,
                       oclc_number TEXT,
                       title TEXT NOT NULL,
                       subtitle TEXT,
                       original_title TEXT,
                       authors TEXT[] NOT NULL DEFAULT '{}',
                       translators TEXT[],
                       illustrators TEXT[],
                       publisher TEXT,
                       publish_date DATE,
                       original_publish_date DATE,
                       edition_number TEXT,
                       printing_number TEXT,
                       original_edition TEXT,
                       is_first_edition BOOLEAN DEFAULT FALSE,
                       collection_name TEXT,
                       volume_in_collection INTEGER,
                       series_name TEXT,
                       volume_in_series INTEGER,
                       book_format TEXT,
                       page_count INTEGER,
                       dimensions TEXT,
                       weight TEXT,
                       language TEXT,
                       original_language TEXT,
                       subjects TEXT[],
                       genres TEXT[],
                       target_audience TEXT,
                       description TEXT,
                       table_of_contents TEXT,
                       cover_url TEXT,
                       purchase_date DATE,
                       purchase_price NUMERIC(10, 2),
                       store_or_vendor TEXT,
                       acquisition_type TEXT,
                       location_property TEXT,
                       location_room TEXT,
                       location_bookcase TEXT,
                       location_shelf TEXT,
                       location_position INTEGER,
                       condition_state TEXT,
                       is_loaned BOOLEAN DEFAULT FALSE,
                       loaned_to TEXT,
                       loan_date TIMESTAMPTZ,
                       expected_return_date TIMESTAMPTZ,
                       created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER set_timestamp_books
    BEFORE UPDATE ON books
    FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

-- Subjective user data and reading tracking
CREATE TABLE user_book_interactions (
                                        user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                                        book_id UUID NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                                        read_status TEXT DEFAULT 'unread',
                                        rating INTEGER CHECK (rating >= 0 AND rating <= 10),
                                        personal_notes TEXT,
                                        reading_notes TEXT,
                                        date_started DATE,
                                        date_finished DATE,
                                        updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                                        PRIMARY KEY (user_id, book_id)
);

CREATE TRIGGER set_timestamp_interactions
    BEFORE UPDATE ON user_book_interactions
    FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

-- Performance indexes
CREATE INDEX idx_books_library ON books(library_id);
CREATE INDEX idx_books_title ON books USING gin (title gin_trgm_ops);
CREATE INDEX idx_books_authors ON books USING gin (authors);
CREATE INDEX idx_user_book_interactions_status ON user_book_interactions(read_status);